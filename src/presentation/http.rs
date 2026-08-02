use std::{sync::Arc, time::Duration};

use askama::Template;
use axum::{
    Extension, Form, Json, Router,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, patch, post},
};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::sync::Semaphore;
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
        config::{AuthConfig, HttpConfig},
        instrument_provider::{CachedInstrumentProvider, InstrumentProviderConfig},
        portfolio_repository::PortfolioRepository,
        security::{IpRateLimiter, LoginRateLimiter},
        security_events,
    },
    presentation::client_ip::{ClientIp, TrustedProxies, resolve_client_ip},
};

const REQUEST_ID_HEADER: &str = "x-request-id";
const SESSION_COOKIE_NAME: &str = "investment_session";
const CSRF_COOKIE_NAME: &str = "investment_csrf";
const CSRF_HEADER_NAME: &str = "x-csrf-token";
const GLOBAL_RATE_LIMIT_SCOPE: &str = "global";
const REGISTER_RATE_LIMIT_SCOPE: &str = "register";
const MUTATION_RATE_LIMIT_SCOPE: &str = "portfolio_mutation";
const CSP_POLICY: &str = concat!(
    "default-src 'self'; ",
    "base-uri 'self'; ",
    "object-src 'none'; ",
    "frame-ancestors 'none'; ",
    "form-action 'self'; ",
    "script-src 'self'; ",
    "style-src 'self'; ",
    "font-src 'self'; ",
    "img-src 'self' data:; ",
    "connect-src 'self'"
);

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth: AuthConfig,
    pub global_rate_limiter: IpRateLimiter,
    pub login_rate_limiter: LoginRateLimiter,
    pub register_rate_limiter: IpRateLimiter,
    pub mutation_rate_limiter: IpRateLimiter,
    pub concurrency_limiter: Arc<Semaphore>,
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
    correlation_id: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
struct CreateBrokerRequest {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateBrokerRequest {
    name: String,
    version: i64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateAssetRequest {
    symbol: String,
    name: String,
    market: String,
    category: String,
    currency: String,
    current_price: Decimal,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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

#[derive(Template)]
#[template(path = "brokers.html")]
struct BrokersTemplate {
    user: PageUser,
    csrf_token: String,
}

#[derive(Template)]
#[template(path = "assets.html")]
struct AssetsTemplate {
    user: PageUser,
    csrf_token: String,
}

#[derive(Template)]
#[template(path = "transactions.html")]
struct TransactionsTemplate {
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
    currency: String,
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
    http: HttpConfig,
    auth: AuthConfig,
    instrument_provider_config: InstrumentProviderConfig,
) -> Router {
    let request_id_header = HeaderName::from_static(REQUEST_ID_HEADER);
    let trusted_proxies = TrustedProxies::new(http.trusted_proxy_ips.iter().copied());
    let global_rate_limiter = IpRateLimiter::new(
        auth.global_rate_limit_max_requests,
        Duration::from_secs(auth.global_rate_limit_window_seconds),
        Duration::from_secs(auth.global_rate_limit_block_seconds),
    );
    let login_rate_limiter = LoginRateLimiter::new(
        auth.login_rate_limit_max_attempts,
        Duration::from_secs(auth.login_rate_limit_window_seconds),
        Duration::from_secs(auth.login_rate_limit_block_seconds),
    );
    let register_rate_limiter = IpRateLimiter::new(
        auth.register_rate_limit_max_requests,
        Duration::from_secs(auth.register_rate_limit_window_seconds),
        Duration::from_secs(auth.register_rate_limit_block_seconds),
    );
    let mutation_rate_limiter = IpRateLimiter::new(
        auth.mutation_rate_limit_max_requests,
        Duration::from_secs(auth.mutation_rate_limit_window_seconds),
        Duration::from_secs(auth.mutation_rate_limit_block_seconds),
    );

    let state = AppState {
        pool,
        auth,
        global_rate_limiter,
        login_rate_limiter,
        register_rate_limiter,
        mutation_rate_limiter,
        concurrency_limiter: Arc::new(Semaphore::new(http.max_concurrent_requests)),
        instrument_provider: CachedInstrumentProvider::new(instrument_provider_config),
    };

    let router = Router::new()
        .route("/", get(root))
        .route("/login", get(login_page).post(login_form))
        .route("/register", get(register_page).post(register_form))
        .route("/logout", post(logout_form))
        .route("/dashboard", get(dashboard_page))
        .route("/brokers", get(brokers_page))
        .route("/assets", get(assets_page))
        .route("/transactions", get(transactions_page))
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
        .nest_service("/static", ServeDir::new("static"));

    #[cfg(test)]
    let router = router.route("/__test/slow", get(test_slow_endpoint));

    router
        .with_state(state.clone())
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            http.request_timeout,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            concurrency_limit,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state,
            global_rate_limit,
        ))
        .layer(axum::middleware::from_fn(security_headers))
        .layer(RequestBodyLimitLayer::new(http.max_body_bytes))
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn_with_state(
            trusted_proxies,
            resolve_client_ip,
        ))
}

async fn root() -> Redirect {
    Redirect::to("/dashboard")
}

async fn global_rate_limit(
    State(state): State<AppState>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = request.uri().path();
    if !is_global_rate_limit_exempt(path)
        && let Err(error) = state
            .global_rate_limiter
            .check_and_record(client_ip, GLOBAL_RATE_LIMIT_SCOPE)
            .await
    {
        security_events::log_http_rate_limited(client_ip, GLOBAL_RATE_LIMIT_SCOPE, path);
        return Err(error);
    }

    Ok(next.run(request).await)
}

async fn concurrency_limit(
    State(state): State<AppState>,
    Extension(ClientIp(client_ip)): Extension<ClientIp>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let Ok(_permit) = state.concurrency_limiter.clone().try_acquire_owned() else {
        security_events::log_concurrency_saturated(client_ip, request.uri().path());
        return Err(AppError::Unavailable);
    };

    Ok(next.run(request).await)
}

fn is_global_rate_limit_exempt(path: &str) -> bool {
    matches!(path, "/health/live" | "/health/ready")
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
    render_authenticated_template(&state, &headers, |user, csrf_token| DashboardTemplate {
        user,
        csrf_token,
    })
    .await
}

async fn brokers_page(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    render_authenticated_template(&state, &headers, |user, csrf_token| BrokersTemplate {
        user,
        csrf_token,
    })
    .await
}

async fn assets_page(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    render_authenticated_template(&state, &headers, |user, csrf_token| AssetsTemplate {
        user,
        csrf_token,
    })
    .await
}

async fn transactions_page(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    render_authenticated_template(&state, &headers, |user, csrf_token| TransactionsTemplate {
        user,
        csrf_token,
    })
    .await
}

async fn render_authenticated_template<T>(
    state: &AppState,
    headers: &HeaderMap,
    template: impl FnOnce(PageUser, String) -> T,
) -> Result<Response, AppError>
where
    T: Template,
{
    let user = match authenticated_user(state, headers).await {
        Ok(user) => user,
        Err(AppError::Unauthorized | AppError::InvalidCredentials) => {
            return Ok(Redirect::to("/login").into_response());
        }
        Err(error) => return Err(error),
    };

    let page_user = PageUser {
        initials: username_initials(&user.username),
        username: user.username,
    };
    let csrf_token = extract_cookie(headers, CSRF_COOKIE_NAME)
        .unwrap_or_default()
        .to_owned();

    Ok(render_template(template(page_user, csrf_token))?.into_response())
}

async fn login_form(
    State(state): State<AppState>,
    Extension(client_ip): Extension<ClientIp>,
    Form(payload): Form<AuthRequest>,
) -> Result<Response, AppError> {
    let issued = issue_login_session(&state, client_ip, payload).await?;
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
    Extension(client_ip): Extension<ClientIp>,
    Form(payload): Form<AuthRequest>,
) -> Result<Response, AppError> {
    apply_register_rate_limit(&state, client_ip).await?;

    let repository = AuthRepository::new(state.pool.clone());
    auth::register_user(
        &repository,
        payload.username.clone(),
        payload.password.clone(),
    )
    .await?;

    login_form(State(state), Extension(client_ip), Form(payload)).await
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

#[cfg(test)]
async fn test_slow_endpoint() -> StatusCode {
    tokio::time::sleep(Duration::from_millis(200)).await;
    StatusCode::OK
}

async fn readiness(State(state): State<AppState>) -> Result<Json<HealthStatus>, AppError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|error| {
            security_events::log_db_readiness_failed(&error);
            AppError::Unavailable
        })?;

    Ok(Json(HealthStatus::ready()))
}

async fn register(
    State(state): State<AppState>,
    Extension(client_ip): Extension<ClientIp>,
    Json(payload): Json<AuthRequest>,
) -> Result<(StatusCode, Json<AuthUserResponse>), AppError> {
    apply_register_rate_limit(&state, client_ip).await?;

    let repository = AuthRepository::new(state.pool.clone());
    let user = auth::register_user(&repository, payload.username, payload.password).await?;

    Ok((StatusCode::CREATED, Json(AuthUserResponse { user })))
}

async fn login(
    State(state): State<AppState>,
    Extension(client_ip): Extension<ClientIp>,
    Json(payload): Json<AuthRequest>,
) -> Result<Response, AppError> {
    let issued = issue_login_session(&state, client_ip, payload).await?;

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
    ClientIp(client_ip): ClientIp,
    payload: AuthRequest,
) -> Result<IssuedSession, AppError> {
    let limiter_username = limiter_username(&payload.username);
    if let Err(error) = state
        .login_rate_limiter
        .check(client_ip, &limiter_username)
        .await
    {
        security_events::log_login_rate_limited(client_ip, &limiter_username);
        return Err(error);
    }

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
                .record_failure(client_ip, &limiter_username)
                .await;
            security_events::log_login_failed(client_ip, &limiter_username);
            return Err(AppError::InvalidCredentials);
        }
        Err(error) => return Err(error),
    };
    state
        .login_rate_limiter
        .record_success(client_ip, &limiter_username)
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

async fn apply_register_rate_limit(
    state: &AppState,
    ClientIp(client_ip): ClientIp,
) -> Result<(), AppError> {
    state
        .register_rate_limiter
        .check_and_record(client_ip, REGISTER_RATE_LIMIT_SCOPE)
        .await
        .inspect_err(|_| {
            security_events::log_http_rate_limited(client_ip, REGISTER_RATE_LIMIT_SCOPE, "");
        })
}

async fn apply_mutation_rate_limit(
    state: &AppState,
    ClientIp(client_ip): ClientIp,
) -> Result<(), AppError> {
    state
        .mutation_rate_limiter
        .check_and_record(client_ip, MUTATION_RATE_LIMIT_SCOPE)
        .await
        .inspect_err(|_| {
            security_events::log_http_rate_limited(client_ip, MUTATION_RATE_LIMIT_SCOPE, "");
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
    Extension(client_ip): Extension<ClientIp>,
    headers: HeaderMap,
    Json(payload): Json<CreateBrokerRequest>,
) -> Result<(StatusCode, Json<PublicBroker>), AppError> {
    apply_mutation_rate_limit(&state, client_ip).await?;
    validate_csrf(&headers, &state.auth)?;
    let user = authenticated_user(&state, &headers).await?;
    let repository = PortfolioRepository::new(state.pool);
    let broker = portfolio::create_broker(&repository, user.id, payload.name).await?;

    Ok((StatusCode::CREATED, Json(broker)))
}

async fn update_broker(
    State(state): State<AppState>,
    Extension(client_ip): Extension<ClientIp>,
    headers: HeaderMap,
    Path(broker_id): Path<Uuid>,
    Json(payload): Json<UpdateBrokerRequest>,
) -> Result<Json<PublicBroker>, AppError> {
    apply_mutation_rate_limit(&state, client_ip).await?;
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
    Extension(client_ip): Extension<ClientIp>,
    headers: HeaderMap,
    Path(broker_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    apply_mutation_rate_limit(&state, client_ip).await?;
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
    Extension(client_ip): Extension<ClientIp>,
    headers: HeaderMap,
    Json(payload): Json<CreateAssetRequest>,
) -> Result<(StatusCode, Json<PublicAsset>), AppError> {
    apply_mutation_rate_limit(&state, client_ip).await?;
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
    Extension(client_ip): Extension<ClientIp>,
    headers: HeaderMap,
    Path(asset_id): Path<Uuid>,
    Json(payload): Json<UpdateAssetRequest>,
) -> Result<Json<PublicAsset>, AppError> {
    apply_mutation_rate_limit(&state, client_ip).await?;
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
    Extension(client_ip): Extension<ClientIp>,
    headers: HeaderMap,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<(StatusCode, Json<PublicTransaction>), AppError> {
    apply_mutation_rate_limit(&state, client_ip).await?;
    record_transaction(state, headers, payload, TransactionType::Buy).await
}

async fn record_sale(
    State(state): State<AppState>,
    Extension(client_ip): Extension<ClientIp>,
    headers: HeaderMap,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<(StatusCode, Json<PublicTransaction>), AppError> {
    apply_mutation_rate_limit(&state, client_ip).await?;
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
                    (currency, date),
                    DailyCashFlow {
                        purchases,
                        sales,
                        fees,
                        net_flow,
                    },
                )| DailyCashFlowResponse {
                    currency: currency.as_str().to_owned(),
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

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    insert_security_headers(&mut response);

    response
}

fn insert_security_headers(response: &mut Response) {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP_POLICY),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Validation(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                message,
            ),
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

        let correlation_id = Uuid::new_v4().to_string();
        let mut response = (
            status,
            Json(ErrorBody {
                error: PublicError {
                    code,
                    message,
                    correlation_id: correlation_id.clone(),
                },
            }),
        )
            .into_response();
        if let Ok(header_value) = HeaderValue::from_str(&correlation_id) {
            response
                .headers_mut()
                .insert(HeaderName::from_static(REQUEST_ID_HEADER), header_value);
        }
        if status.is_server_error() {
            security_events::log_server_error(status.as_u16(), code, &correlation_id);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::extract::ConnectInfo;
    use serde_json::Value;
    use sqlx::postgres::PgPoolOptions;
    use std::net::{IpAddr, SocketAddr};
    use tower::ServiceExt;

    const TEST_DATABASE_URL: &str =
        "postgres://carteira_runtime:carteira_runtime_dev_password@127.0.0.1:65432/carteira_dev";
    const DEV_DATABASE_URL: &str =
        "postgres://carteira_runtime:carteira_runtime_dev_password@127.0.0.1:5433/carteira_dev";

    fn auth_config() -> AuthConfig {
        AuthConfig {
            jwt_secret: "test-jwt-secret-with-at-least-32-characters".to_owned(),
            jwt_issuer: "carteira-de-investimentos-maicaosa".to_owned(),
            jwt_audience: "carteira-web".to_owned(),
            session_hash_key: "test-session-hash-key-with-at-least-32-chars".to_owned(),
            session_ttl_seconds: 3_600,
            cookie_secure: false,
            allowed_origins: vec!["http://127.0.0.1:3000".to_owned()],
            global_rate_limit_max_requests: 300,
            global_rate_limit_window_seconds: 60,
            global_rate_limit_block_seconds: 300,
            login_rate_limit_max_attempts: 5,
            login_rate_limit_window_seconds: 300,
            login_rate_limit_block_seconds: 900,
            register_rate_limit_max_requests: 3,
            register_rate_limit_window_seconds: 300,
            register_rate_limit_block_seconds: 900,
            mutation_rate_limit_max_requests: 60,
            mutation_rate_limit_window_seconds: 60,
            mutation_rate_limit_block_seconds: 300,
            expired_session_retention_days: 7,
        }
    }

    fn http_config() -> HttpConfig {
        HttpConfig {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 3000)),
            trusted_proxy_ips: Vec::new(),
            request_timeout: Duration::from_secs(5),
            max_body_bytes: 16_384,
            max_concurrent_requests: 16,
        }
    }

    fn instrument_provider_config() -> InstrumentProviderConfig {
        InstrumentProviderConfig {
            timeout: Duration::from_secs(1),
            cache_ttl: Duration::from_secs(60),
            stale_ttl: Duration::from_secs(300),
            max_results: 5,
        }
    }

    fn lazy_pool() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(100))
            .connect_lazy(TEST_DATABASE_URL)
            .expect("lazy test database pool")
    }

    async fn saturated_pool() -> (PgPool, sqlx::pool::PoolConnection<sqlx::Postgres>) {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| DEV_DATABASE_URL.to_owned());
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(2))
            .connect_lazy(&database_url)
            .expect("lazy database pool for saturation test");
        let held_connection = pool.acquire().await.expect("hold only database connection");

        (pool, held_connection)
    }

    fn test_router(auth: AuthConfig) -> Router {
        build_router(
            lazy_pool(),
            http_config(),
            auth,
            instrument_provider_config(),
        )
    }

    fn request(method: axum::http::Method, uri: &str, body: Body) -> Request {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .expect("build test request");
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 31_000))));
        request
    }

    fn proxied_request(peer: IpAddr, client_ip: &str, uri: &str) -> Request {
        let mut request = request(axum::http::Method::GET, uri, Body::empty());
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(peer, 31_000)));
        request.headers_mut().insert(
            HeaderName::from_static("x-real-ip"),
            HeaderValue::from_str(client_ip).expect("valid client IP header"),
        );
        request
    }

    async fn json_body(response: Response) -> Value {
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("read response body");
        serde_json::from_slice(&body).expect("response body is json")
    }

    async fn assert_public_error(response: Response, status: StatusCode, code: &str) {
        assert_eq!(response.status(), status);
        let header_correlation_id = response
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .expect("error response includes x-request-id");
        let json = json_body(response).await;
        assert_eq!(json["error"]["code"], code);
        assert!(json["error"]["message"].as_str().is_some());
        assert_eq!(
            json["error"]["correlation_id"].as_str(),
            Some(header_correlation_id.as_str())
        );
        assert!(Uuid::parse_str(&header_correlation_id).is_ok());
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

    #[test]
    fn validation_errors_use_unprocessable_entity() {
        assert_eq!(
            AppError::Validation("campo inválido")
                .into_response()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn security_headers_include_csp_and_no_store() {
        let mut response = StatusCode::OK.into_response();
        insert_security_headers(&mut response);

        assert_eq!(
            response.headers().get(header::CONTENT_SECURITY_POLICY),
            Some(&HeaderValue::from_static(CSP_POLICY))
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store, max-age=0"))
        );
        assert_eq!(
            response.headers().get(header::X_CONTENT_TYPE_OPTIONS),
            Some(&HeaderValue::from_static("nosniff"))
        );
    }

    #[test]
    fn dashboard_template_escapes_user_controlled_values() {
        let html = DashboardTemplate {
            user: PageUser {
                username: "<script>alert(1)</script>".to_owned(),
                initials: "<X".to_owned(),
            },
            csrf_token: "\"><script>alert(2)</script>".to_owned(),
        }
        .render()
        .expect("render dashboard template");

        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(!html.contains("<script>alert(2)</script>"));
        assert!(html.contains("alert(1)"));
        assert!(html.contains("alert(2)"));
        assert!(html.contains("&lt;") || html.contains("&#60;"));
        assert!(html.contains("&quot;") || html.contains("&#34;"));
    }

    #[test]
    fn form_templates_expose_accessible_error_targets() {
        fn page_user() -> PageUser {
            PageUser {
                username: "maicom".to_owned(),
                initials: "M".to_owned(),
            }
        }

        let templates = [
            LoginTemplate.render().expect("render login template"),
            RegisterTemplate.render().expect("render register template"),
            BrokersTemplate {
                user: page_user(),
                csrf_token: "csrf".to_owned(),
            }
            .render()
            .expect("render brokers template"),
            AssetsTemplate {
                user: page_user(),
                csrf_token: "csrf".to_owned(),
            }
            .render()
            .expect("render assets template"),
            TransactionsTemplate {
                user: page_user(),
                csrf_token: "csrf".to_owned(),
            }
            .render()
            .expect("render transactions template"),
        ];

        for html in templates {
            assert!(html.contains("data-form-status"));
            assert!(html.contains("role=\"alert\""));
            assert!(html.contains("data-error-for="));
            assert!(html.contains("aria-describedby="));
        }
    }

    #[tokio::test]
    async fn internal_error_response_is_generic_and_stable() {
        let response = AppError::Internal.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("read error response body");
        let json: serde_json::Value =
            serde_json::from_slice(&body).expect("error response is json");

        assert_eq!(json["error"]["code"], "internal_error");
        assert_eq!(json["error"]["message"], "erro interno");
        assert!(
            json["error"]["correlation_id"]
                .as_str()
                .and_then(|value| Uuid::parse_str(value).ok())
                .is_some()
        );
    }

    #[tokio::test]
    async fn http_get_authenticated_api_without_session_returns_401() {
        let response = test_router(auth_config())
            .oneshot(request(
                axum::http::Method::GET,
                "/api/assets",
                Body::empty(),
            ))
            .await
            .expect("route response");

        assert_public_error(response, StatusCode::UNAUTHORIZED, "unauthorized").await;
    }

    #[tokio::test]
    async fn http_mutation_without_csrf_returns_403_before_authentication() {
        let payload = serde_json::json!({
            "symbol": "PETR4",
            "name": "Petrobras PN",
            "market": "B3",
            "category": "STOCK",
            "currency": "BRL",
            "current_price": "38.42"
        });
        let response = test_router(auth_config())
            .oneshot(request(
                axum::http::Method::POST,
                "/api/assets",
                Body::from(payload.to_string()),
            ))
            .await
            .expect("route response");

        assert_public_error(response, StatusCode::FORBIDDEN, "forbidden").await;
    }

    #[tokio::test]
    async fn http_invalid_json_payload_returns_422() {
        let response = test_router(auth_config())
            .oneshot(request(
                axum::http::Method::POST,
                "/auth/register",
                Body::from(
                    serde_json::json!({
                        "username": "x",
                        "password": "short"
                    })
                    .to_string(),
                ),
            ))
            .await
            .expect("route response");

        assert_public_error(
            response,
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_error",
        )
        .await;
    }

    #[tokio::test]
    async fn http_global_rate_limit_returns_429_and_recovers_by_scope() {
        let mut auth = auth_config();
        auth.global_rate_limit_max_requests = 1;
        auth.global_rate_limit_window_seconds = 60;
        auth.global_rate_limit_block_seconds = 60;
        let router = test_router(auth);

        let first = router
            .clone()
            .oneshot(request(axum::http::Method::GET, "/login", Body::empty()))
            .await
            .expect("first route response");
        assert_eq!(first.status(), StatusCode::OK);

        let second = router
            .oneshot(request(axum::http::Method::GET, "/login", Body::empty()))
            .await
            .expect("second route response");

        assert_public_error(second, StatusCode::TOO_MANY_REQUESTS, "rate_limited").await;
    }

    #[tokio::test]
    async fn trusted_proxy_clients_use_independent_rate_limit_buckets() {
        let proxy_ip = IpAddr::from([192, 0, 2, 10]);
        let mut http = http_config();
        http.trusted_proxy_ips = vec![proxy_ip];
        let mut auth = auth_config();
        auth.global_rate_limit_max_requests = 1;
        let router = build_router(lazy_pool(), http, auth, instrument_provider_config());

        let first_client = router
            .clone()
            .oneshot(proxied_request(proxy_ip, "198.51.100.10", "/login"))
            .await
            .expect("first client response");
        assert_eq!(first_client.status(), StatusCode::OK);

        let second_client = router
            .clone()
            .oneshot(proxied_request(proxy_ip, "198.51.100.11", "/login"))
            .await
            .expect("second client response");
        assert_eq!(second_client.status(), StatusCode::OK);

        let repeated_first_client = router
            .oneshot(proxied_request(proxy_ip, "198.51.100.10", "/login"))
            .await
            .expect("repeated first client response");
        assert_public_error(
            repeated_first_client,
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
        )
        .await;
    }

    #[tokio::test]
    async fn http_concurrency_saturation_returns_controlled_503() {
        let mut http = http_config();
        http.max_concurrent_requests = 1;
        let router = build_router(
            lazy_pool(),
            http,
            auth_config(),
            instrument_provider_config(),
        );

        let first_router = router.clone();
        let first_request = request(axum::http::Method::GET, "/__test/slow", Body::empty());
        let first_response = tokio::spawn(async move { first_router.oneshot(first_request).await });

        tokio::time::sleep(Duration::from_millis(25)).await;

        let second = router
            .oneshot(request(
                axum::http::Method::GET,
                "/__test/slow",
                Body::empty(),
            ))
            .await
            .expect("second route response");

        assert_public_error(
            second,
            StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
        )
        .await;

        let first = first_response
            .await
            .expect("first request task completed")
            .expect("first route response");
        assert_eq!(first.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn http_readiness_with_unavailable_database_returns_controlled_503() {
        let response = test_router(auth_config())
            .oneshot(request(
                axum::http::Method::GET,
                "/health/ready",
                Body::empty(),
            ))
            .await
            .expect("route response");

        assert_public_error(
            response,
            StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
        )
        .await;
    }

    #[tokio::test]
    async fn http_readiness_with_saturated_database_pool_returns_controlled_503() {
        let (pool, _held_connection) = saturated_pool().await;
        let response = build_router(
            pool,
            http_config(),
            auth_config(),
            instrument_provider_config(),
        )
        .oneshot(request(
            axum::http::Method::GET,
            "/health/ready",
            Body::empty(),
        ))
        .await
        .expect("route response");

        assert_public_error(
            response,
            StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
        )
        .await;
    }

    #[tokio::test]
    async fn http_internal_error_keeps_generic_500_envelope() {
        async fn forced_internal_error() -> Result<StatusCode, AppError> {
            Err(AppError::Internal)
        }

        let response = Router::new()
            .route("/forced-internal-error", get(forced_internal_error))
            .oneshot(
                Request::builder()
                    .uri("/forced-internal-error")
                    .body(Body::empty())
                    .expect("build test request"),
            )
            .await
            .expect("route response");

        assert_public_error(
            response,
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
        )
        .await;
    }

    #[test]
    fn global_rate_limit_exempts_health_checks_only() {
        assert!(is_global_rate_limit_exempt("/health/live"));
        assert!(is_global_rate_limit_exempt("/health/ready"));
        assert!(!is_global_rate_limit_exempt("/login"));
        assert!(!is_global_rate_limit_exempt("/static/app.css"));
    }

    #[test]
    fn asset_request_rejects_unknown_fields_to_reduce_mass_assignment_risk() {
        let payload = serde_json::json!({
            "symbol": "PETR4",
            "name": "Petrobras PN",
            "market": "B3",
            "category": "STOCK",
            "currency": "BRL",
            "current_price": "38.42",
            "user_id": Uuid::new_v4(),
            "is_admin": true
        });

        assert!(serde_json::from_value::<CreateAssetRequest>(payload).is_err());
    }

    #[test]
    fn transaction_request_rejects_unknown_fields_to_reduce_mass_assignment_risk() {
        let payload = serde_json::json!({
            "asset_id": Uuid::new_v4(),
            "broker_id": Uuid::new_v4(),
            "quantity": "10",
            "unit_price": "38.42",
            "fees": "0",
            "force_user_id": Uuid::new_v4()
        });

        assert!(serde_json::from_value::<CreateTransactionRequest>(payload).is_err());
    }
}
