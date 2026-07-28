use std::{net::SocketAddr, time::Duration};

use askama::Template;
use axum::{
    Form, Json, Router,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, patch, post},
};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use tower_http::{
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::{
    application::{
        auth::{self, PublicUser},
        error::AppError,
        instrument,
        portfolio::{
            self, AssetDraft, AssetPatch, BrokerPatch, PublicAsset, PublicBroker,
            PublicTransaction, TransactionDraft,
        },
    },
    domain::{
        health::HealthStatus,
        portfolio::{DailyCashFlow, PortfolioSummary, TransactionType},
        user::NormalizedUsername,
    },
    infrastructure::{
        auth_repository::AuthRepository,
        config::AuthConfig,
        instrument_provider::{CachedInstrumentProvider, InstrumentProviderConfig},
        portfolio_repository::PortfolioRepository,
        security::LoginRateLimiter,
    },
};

const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_BODY_BYTES: usize = 1024 * 1024;
const SESSION_COOKIE_NAME: &str = "investment_session";
const CSRF_COOKIE_NAME: &str = "investment_csrf";
const CSRF_HEADER_NAME: &str = "x-csrf-token";

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth: AuthConfig,
    pub login_rate_limiter: LoginRateLimiter,
    pub instrument_provider: CachedInstrumentProvider,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: PublicError,
}

#[derive(Debug, Serialize)]
struct PublicError {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug, serde::Deserialize)]
struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Debug, serde::Deserialize)]
struct LogoutForm {
    csrf_token: String,
}

#[derive(Debug, Serialize)]
struct AuthUserResponse {
    user: PublicUser,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    user: PublicUser,
    csrf_token: String,
}

struct IssuedSession {
    user: PublicUser,
    csrf_token: String,
    session_cookie: HeaderValue,
    csrf_cookie: HeaderValue,
}

#[derive(Debug, serde::Deserialize)]
struct CreateBrokerRequest {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateBrokerRequest {
    name: String,
    version: i64,
}

#[derive(Debug, serde::Deserialize)]
struct CreateAssetRequest {
    symbol: String,
    name: String,
    market: String,
    category: String,
    currency: String,
    current_price: Decimal,
}

#[derive(Debug, serde::Deserialize)]
struct UpdateAssetRequest {
    symbol: Option<String>,
    name: Option<String>,
    market: Option<String>,
    category: Option<String>,
    currency: Option<String>,
    current_price: Option<Decimal>,
    version: i64,
}

#[derive(Debug, serde::Deserialize)]
struct CreateTransactionRequest {
    asset_id: Uuid,
    broker_id: Uuid,
    quantity: Decimal,
    unit_price: Decimal,
    fees: Option<Decimal>,
    occurred_at_unix: Option<i64>,
    notes: Option<String>,
}

#[derive(Debug, Serialize)]
struct BrokerListResponse {
    brokers: Vec<PublicBroker>,
}

#[derive(Debug, Serialize)]
struct AssetListResponse {
    assets: Vec<PublicAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct InstrumentSearchRequest {
    q: String,
}

#[derive(Debug)]
struct PageUser {
    username: String,
    initials: String,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate;

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate;

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    user: PageUser,
    csrf_token: String,
}

#[derive(Debug, Serialize)]
struct TransactionListResponse {
    transactions: Vec<PublicTransaction>,
}

#[derive(Debug, Serialize)]
struct PortfolioSummaryResponse {
    positions: Vec<PositionResponse>,
    totals_by_currency: Vec<CurrencyTotalResponse>,
    allocation_by_category: Vec<CategoryAllocationResponse>,
    allocation_by_broker: Vec<BrokerAllocationResponse>,
    daily_cash_flow: Vec<DailyCashFlowResponse>,
}

#[derive(Debug, Serialize)]
struct PositionResponse {
    asset_id: Uuid,
    broker_id: Uuid,
    currency: String,
    category: String,
    #[serde(with = "rust_decimal::serde::str")]
    quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    cost_basis: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    average_cost: Decimal,
}

#[derive(Debug, Serialize)]
struct CurrencyTotalResponse {
    currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    total: Decimal,
}

#[derive(Debug, Serialize)]
struct CategoryAllocationResponse {
    currency: String,
    category: String,
    #[serde(with = "rust_decimal::serde::str")]
    total: Decimal,
}

#[derive(Debug, Serialize)]
struct BrokerAllocationResponse {
    currency: String,
    broker_id: Uuid,
    #[serde(with = "rust_decimal::serde::str")]
    total: Decimal,
}

#[derive(Debug, Serialize)]
struct DailyCashFlowResponse {
    date: String,
    #[serde(with = "rust_decimal::serde::str")]
    purchases: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    sales: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    fees: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    net_flow: Decimal,
}

pub fn build_router(
    pool: PgPool,
    auth: AuthConfig,
    instrument_provider_config: InstrumentProviderConfig,
) -> Router {
    let request_id_header = HeaderName::from_static(REQUEST_ID_HEADER);
    let login_rate_limiter = LoginRateLimiter::new(
        auth.login_rate_limit_max_attempts,
        Duration::from_secs(auth.login_rate_limit_window_seconds),
        Duration::from_secs(auth.login_rate_limit_block_seconds),
    );

    Router::new()
        .route("/", get(root))
        .route("/login", get(login_page).post(login_form))
        .route("/register", get(register_page).post(register_form))
        .route("/logout", post(logout_form))
        .route("/dashboard", get(dashboard_page))
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/logout", post(logout))
        .route("/api/brokers", get(list_brokers).post(create_broker))
        .route("/api/brokers/{broker_id}", patch(update_broker))
        .route("/api/brokers/{broker_id}/archive", post(archive_broker))
        .route("/api/assets", get(list_assets).post(create_asset))
        .route("/api/assets/{asset_id}", patch(update_asset))
        .route("/api/instruments/search", get(search_instruments))
        .route("/api/transactions", get(list_transactions))
        .route("/api/transactions/buy", post(record_purchase))
        .route("/api/transactions/sell", post(record_sale))
        .route("/api/portfolio/summary", get(portfolio_summary))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(AppState {
            pool,
            auth,
            login_rate_limiter,
            instrument_provider: CachedInstrumentProvider::new(instrument_provider_config),
        })
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
}

async fn root() -> Redirect {
    Redirect::to("/dashboard")
}

async fn login_page() -> Result<Html<String>, AppError> {
    render_template(LoginTemplate)
}

async fn register_page() -> Result<Html<String>, AppError> {
    render_template(RegisterTemplate)
}

async fn dashboard_page(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let user = match authenticated_user(&state, &headers).await {
        Ok(user) => user,
        Err(AppError::Unauthorized | AppError::InvalidCredentials) => {
            return Ok(Redirect::to("/login").into_response());
        }
        Err(error) => return Err(error),
    };

    Ok(render_template(DashboardTemplate {
        user: PageUser {
            initials: username_initials(&user.username),
            username: user.username,
        },
        csrf_token: extract_cookie(&headers, CSRF_COOKIE_NAME)
            .unwrap_or_default()
            .to_owned(),
    })?
    .into_response())
}

async fn login_form(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Form(payload): Form<AuthRequest>,
) -> Result<Response, AppError> {
    let issued = issue_login_session(&state, addr, payload).await?;
    let mut response = Redirect::to("/dashboard").into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, issued.session_cookie);
    response
        .headers_mut()
        .append(header::SET_COOKIE, issued.csrf_cookie);

    Ok(response)
}

async fn register_form(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Form(payload): Form<AuthRequest>,
) -> Result<Response, AppError> {
    let repository = AuthRepository::new(state.pool.clone());
    auth::register_user(
        &repository,
        payload.username.clone(),
        payload.password.clone(),
    )
    .await?;

    login_form(State(state), ConnectInfo(addr), Form(payload)).await
}

async fn logout_form(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(payload): Form<LogoutForm>,
) -> Result<Response, AppError> {
    validate_csrf_value(&headers, &state.auth, &payload.csrf_token)?;

    let token = extract_session_cookie(&headers)?;
    let repository = AuthRepository::new(state.pool.clone());

    auth::logout_user(&repository, &state.auth, token).await?;

    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        build_clear_session_cookie(state.auth.cookie_secure)?,
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        build_clear_csrf_cookie(state.auth.cookie_secure)?,
    );

    Ok(response)
}

async fn liveness() -> Json<HealthStatus> {
    Json(HealthStatus::live())
}

async fn readiness(State(state): State<AppState>) -> Result<Json<HealthStatus>, AppError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "readiness falhou");
            AppError::Unavailable
        })?;

    Ok(Json(HealthStatus::ready()))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<(StatusCode, Json<AuthUserResponse>), AppError> {
    let repository = AuthRepository::new(state.pool.clone());
    let user = auth::register_user(&repository, payload.username, payload.password).await?;

    Ok((StatusCode::CREATED, Json(AuthUserResponse { user })))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthRequest>,
) -> Result<Response, AppError> {
    let issued = issue_login_session(&state, addr, payload).await?;

    let mut response = Json(LoginResponse {
        user: issued.user,
        csrf_token: issued.csrf_token,
    })
    .into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, issued.session_cookie);
    response
        .headers_mut()
        .append(header::SET_COOKIE, issued.csrf_cookie);

    Ok(response)
}

async fn issue_login_session(
    state: &AppState,
    addr: SocketAddr,
    payload: AuthRequest,
) -> Result<IssuedSession, AppError> {
    let limiter_username = limiter_username(&payload.username);
    state
        .login_rate_limiter
        .check(addr.ip(), &limiter_username)
        .await?;

    let repository = AuthRepository::new(state.pool.clone());
    let session_result = match auth::login_user(
        &repository,
        &state.auth,
        payload.username,
        payload.password,
    )
    .await
    {
        Ok(session) => session,
        Err(AppError::InvalidCredentials) => {
            state
                .login_rate_limiter
                .record_failure(addr.ip(), &limiter_username)
                .await;
            return Err(AppError::InvalidCredentials);
        }
        Err(error) => return Err(error),
    };
    state
        .login_rate_limiter
        .record_success(addr.ip(), &limiter_username)
        .await;

    let cookie = build_session_cookie(
        &session_result.token,
        state.auth.session_ttl_seconds,
        state.auth.cookie_secure,
    )?;
    let csrf_token = generate_csrf_token();
    let csrf_cookie = build_csrf_cookie(
        &csrf_token,
        state.auth.session_ttl_seconds,
        state.auth.cookie_secure,
    )?;

    Ok(IssuedSession {
        user: session_result.user,
        csrf_token,
        session_cookie: cookie,
        csrf_cookie,
    })
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthUserResponse>, AppError> {
    let token = extract_session_cookie(&headers)?;
    let repository = AuthRepository::new(state.pool);
    let user = auth::authenticate_token(&repository, &state.auth, token).await?;

    Ok(Json(AuthUserResponse { user }))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, AppError> {
    validate_csrf(&headers, &state.auth)?;

    let token = extract_session_cookie(&headers)?;
    let repository = AuthRepository::new(state.pool);

    auth::logout_user(&repository, &state.auth, token).await?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        build_clear_session_cookie(state.auth.cookie_secure)?,
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        build_clear_csrf_cookie(state.auth.cookie_secure)?,
    );

    Ok(response)
}

async fn list_brokers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BrokerListResponse>, AppError> {
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let brokers = portfolio::list_brokers(&repository, user.id).await?;

    Ok(Json(BrokerListResponse { brokers }))
}

async fn create_broker(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateBrokerRequest>,
) -> Result<(StatusCode, Json<PublicBroker>), AppError> {
    validate_csrf(&headers, &state.auth)?;
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let broker = portfolio::create_broker(&repository, user.id, payload.name).await?;

    Ok((StatusCode::CREATED, Json(broker)))
}

async fn update_broker(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(broker_id): Path<Uuid>,
    Json(payload): Json<UpdateBrokerRequest>,
) -> Result<Json<PublicBroker>, AppError> {
    validate_csrf(&headers, &state.auth)?;
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let broker = portfolio::update_broker(
        &repository,
        user.id,
        broker_id,
        BrokerPatch {
            name: payload.name,
            version: payload.version,
        },
    )
    .await?;

    Ok(Json(broker))
}

async fn archive_broker(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(broker_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    validate_csrf(&headers, &state.auth)?;
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    portfolio::archive_broker(&repository, user.id, broker_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_assets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AssetListResponse>, AppError> {
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let assets = portfolio::list_assets(&repository, user.id).await?;

    Ok(Json(AssetListResponse { assets }))
}

async fn create_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateAssetRequest>,
) -> Result<(StatusCode, Json<PublicAsset>), AppError> {
    validate_csrf(&headers, &state.auth)?;
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let asset = portfolio::create_asset(
        &repository,
        user.id,
        AssetDraft {
            symbol: payload.symbol,
            name: payload.name,
            market: payload.market,
            category: payload.category,
            currency: payload.currency,
            current_price: payload.current_price,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(asset)))
}

async fn update_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(asset_id): Path<Uuid>,
    Json(payload): Json<UpdateAssetRequest>,
) -> Result<Json<PublicAsset>, AppError> {
    validate_csrf(&headers, &state.auth)?;
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let asset = portfolio::update_asset(
        &repository,
        user.id,
        asset_id,
        AssetPatch {
            symbol: payload.symbol,
            name: payload.name,
            market: payload.market,
            category: payload.category,
            currency: payload.currency,
            current_price: payload.current_price,
            version: payload.version,
        },
    )
    .await?;

    Ok(Json(asset))
}

async fn search_instruments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<InstrumentSearchRequest>,
) -> Result<Json<crate::domain::instrument::InstrumentSearchResult>, AppError> {
    authenticated_user(&state, &headers).await?;
    let result = instrument::search_instruments(&state.instrument_provider, &query.q).await?;

    Ok(Json(result))
}

async fn list_transactions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TransactionListResponse>, AppError> {
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let transactions = portfolio::list_transactions(&repository, user.id).await?;

    Ok(Json(TransactionListResponse { transactions }))
}

async fn record_purchase(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<(StatusCode, Json<PublicTransaction>), AppError> {
    record_transaction(state, headers, payload, TransactionType::Buy).await
}

async fn record_sale(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<(StatusCode, Json<PublicTransaction>), AppError> {
    record_transaction(state, headers, payload, TransactionType::Sell).await
}

async fn record_transaction(
    state: AppState,
    headers: HeaderMap,
    payload: CreateTransactionRequest,
    transaction_type: TransactionType,
) -> Result<(StatusCode, Json<PublicTransaction>), AppError> {
    validate_csrf(&headers, &state.auth)?;
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let occurred_at = payload
        .occurred_at_unix
        .map(OffsetDateTime::from_unix_timestamp)
        .transpose()
        .map_err(|_| AppError::Validation("data da operação inválida"))?
        .unwrap_or_else(OffsetDateTime::now_utc);
    let transaction = portfolio::record_transaction(
        &repository,
        user.id,
        TransactionDraft {
            asset_id: payload.asset_id,
            broker_id: payload.broker_id,
            transaction_type,
            quantity: payload.quantity,
            unit_price: payload.unit_price,
            fees: payload.fees.unwrap_or(Decimal::ZERO),
            occurred_at,
            notes: payload.notes,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(transaction)))
}

async fn portfolio_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PortfolioSummaryResponse>, AppError> {
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let summary = portfolio::portfolio_summary(&repository, user.id).await?;

    Ok(Json(summary_response(summary)))
}

async fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<PublicUser, AppError> {
    let token = extract_session_cookie(headers)?;
    let repository = AuthRepository::new(state.pool.clone());

    auth::authenticate_token(&repository, &state.auth, token).await
}

fn extract_session_cookie(headers: &HeaderMap) -> Result<&str, AppError> {
    extract_cookie(headers, SESSION_COOKIE_NAME)
}

fn extract_cookie<'a>(headers: &'a HeaderMap, cookie_name: &str) -> Result<&'a str, AppError> {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    cookie_header
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(name, value)| (name == cookie_name).then_some(value))
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)
}

fn build_session_cookie(
    token: &str,
    max_age_seconds: u64,
    secure: bool,
) -> Result<HeaderValue, AppError> {
    let secure_attribute = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_seconds}{secure_attribute}"
    ))
    .map_err(|_| AppError::Internal)
}

fn build_clear_session_cookie(secure: bool) -> Result<HeaderValue, AppError> {
    let secure_attribute = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE_NAME}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{secure_attribute}"
    ))
    .map_err(|_| AppError::Internal)
}

fn build_csrf_cookie(
    token: &str,
    max_age_seconds: u64,
    secure: bool,
) -> Result<HeaderValue, AppError> {
    let secure_attribute = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{CSRF_COOKIE_NAME}={token}; SameSite=Lax; Path=/; Max-Age={max_age_seconds}{secure_attribute}"
    ))
    .map_err(|_| AppError::Internal)
}

fn build_clear_csrf_cookie(secure: bool) -> Result<HeaderValue, AppError> {
    let secure_attribute = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{CSRF_COOKIE_NAME}=; SameSite=Lax; Path=/; Max-Age=0{secure_attribute}"
    ))
    .map_err(|_| AppError::Internal)
}

fn validate_csrf(headers: &HeaderMap, auth: &AuthConfig) -> Result<(), AppError> {
    validate_origin(headers, auth)?;

    let cookie_token =
        extract_cookie(headers, CSRF_COOKIE_NAME).map_err(|_| AppError::Forbidden)?;
    let header_token = headers
        .get(CSRF_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Forbidden)?;

    validate_csrf_pair(cookie_token, header_token)
}

fn validate_csrf_value(
    headers: &HeaderMap,
    auth: &AuthConfig,
    submitted_token: &str,
) -> Result<(), AppError> {
    validate_origin(headers, auth)?;

    let cookie_token =
        extract_cookie(headers, CSRF_COOKIE_NAME).map_err(|_| AppError::Forbidden)?;

    validate_csrf_pair(cookie_token, submitted_token)
}

fn validate_csrf_pair(cookie_token: &str, submitted_token: &str) -> Result<(), AppError> {
    if cookie_token == submitted_token && cookie_token.len() >= 32 {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn validate_origin(headers: &HeaderMap, auth: &AuthConfig) -> Result<(), AppError> {
    if let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        return auth
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin)
            .then_some(())
            .ok_or(AppError::Forbidden);
    }

    if let Some(referer) = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
    {
        return auth
            .allowed_origins
            .iter()
            .any(|allowed| referer == allowed || referer.starts_with(&format!("{allowed}/")))
            .then_some(())
            .ok_or(AppError::Forbidden);
    }

    Ok(())
}

fn generate_csrf_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn limiter_username(username: &str) -> String {
    NormalizedUsername::parse(username)
        .map(|normalized| normalized.as_str().to_owned())
        .unwrap_or_else(|_| {
            username
                .trim()
                .to_ascii_lowercase()
                .chars()
                .take(64)
                .collect()
        })
}

fn render_template<T: Template>(template: T) -> Result<Html<String>, AppError> {
    template.render().map(Html).map_err(|error| {
        tracing::error!(%error, "falha ao renderizar template");
        AppError::Internal
    })
}

fn username_initials(username: &str) -> String {
    let initials: String = username
        .split(|character: char| !character.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .take(2)
        .filter_map(|part| part.chars().next())
        .flat_map(char::to_uppercase)
        .collect();

    if initials.is_empty() {
        "?".to_owned()
    } else {
        initials.chars().take(2).collect()
    }
}

fn summary_response(summary: PortfolioSummary) -> PortfolioSummaryResponse {
    PortfolioSummaryResponse {
        positions: summary
            .positions
            .into_iter()
            .map(|position| PositionResponse {
                asset_id: position.asset_id,
                broker_id: position.broker_id,
                currency: position.currency.as_str().to_owned(),
                category: position.category.as_str().to_owned(),
                quantity: position.quantity,
                cost_basis: position.cost_basis,
                average_cost: position.average_cost(),
            })
            .collect(),
        totals_by_currency: summary
            .totals_by_currency
            .into_iter()
            .map(|(currency, total)| CurrencyTotalResponse {
                currency: currency.as_str().to_owned(),
                total,
            })
            .collect(),
        allocation_by_category: summary
            .allocation_by_category
            .into_iter()
            .map(|((currency, category), total)| CategoryAllocationResponse {
                currency: currency.as_str().to_owned(),
                category: category.as_str().to_owned(),
                total,
            })
            .collect(),
        allocation_by_broker: summary
            .allocation_by_broker
            .into_iter()
            .map(|((currency, broker_id), total)| BrokerAllocationResponse {
                currency: currency.as_str().to_owned(),
                broker_id,
                total,
            })
            .collect(),
        daily_cash_flow: summary
            .daily_cash_flow
            .into_iter()
            .map(
                |(
                    date,
                    DailyCashFlow {
                        purchases,
                        sales,
                        fees,
                        net_flow,
                    },
                )| DailyCashFlowResponse {
                    date: date.to_string(),
                    purchases,
                    sales,
                    fees,
                    net_flow,
                },
            )
            .collect(),
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Validation(message) => (StatusCode::BAD_REQUEST, "validation_error", message),
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "usuário ou senha inválidos",
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "autenticação necessária",
            ),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "acesso negado"),
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "muitas tentativas; aguarde antes de tentar novamente",
            ),
            AppError::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            AppError::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                "serviço temporariamente indisponível",
            ),
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "erro interno",
            ),
        };

        (
            status,
            Json(ErrorBody {
                error: PublicError { code, message },
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth_config() -> AuthConfig {
        AuthConfig {
            jwt_secret: "test-jwt-secret-with-at-least-32-characters".to_owned(),
            jwt_issuer: "carteira-de-investimentos-maicaosa".to_owned(),
            jwt_audience: "carteira-web".to_owned(),
            session_hash_key: "test-session-hash-key-with-at-least-32-chars".to_owned(),
            session_ttl_seconds: 3_600,
            cookie_secure: false,
            allowed_origins: vec!["http://127.0.0.1:3000".to_owned()],
            login_rate_limit_max_attempts: 5,
            login_rate_limit_window_seconds: 300,
            login_rate_limit_block_seconds: 900,
            expired_session_retention_days: 7,
        }
    }

    #[test]
    fn csrf_accepts_matching_cookie_header_and_allowed_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:3000"),
        );
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("investment_csrf=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );
        headers.insert(
            CSRF_HEADER_NAME,
            HeaderValue::from_static("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );

        assert!(validate_csrf(&headers, &auth_config()).is_ok());
    }

    #[test]
    fn csrf_rejects_cross_site_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.example"),
        );
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("investment_csrf=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );
        headers.insert(
            CSRF_HEADER_NAME,
            HeaderValue::from_static("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );

        assert!(matches!(
            validate_csrf(&headers, &auth_config()),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn csrf_rejects_missing_header_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("investment_csrf=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );

        assert!(matches!(
            validate_csrf(&headers, &auth_config()),
            Err(AppError::Forbidden)
        ));
    }

    #[test]
    fn username_initials_uses_two_name_parts() {
        assert_eq!(username_initials("maicom dev"), "MD");
    }

    #[test]
    fn username_initials_falls_back_for_blank_value() {
        assert_eq!(username_initials("  "), "?");
    }
}
