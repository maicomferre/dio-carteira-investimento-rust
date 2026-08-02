import { confirmAction, showError, showSuccess } from "./alerts.js";
import { bindAccessibleValidation, clearFormErrors, ensureFormValidity, showFormError } from "./forms.js";
import { HttpClient } from "./http.js";

interface Broker {
  id: string;
  name: string;
  is_archived: boolean;
  version: number;
}

interface Asset {
  id: string;
  symbol: string;
  name: string;
  market: string;
  category: string;
  currency: string;
  current_price: string;
  version: number;
}

interface Transaction {
  id: string;
  asset_id: string;
  broker_id: string;
  transaction_type: string;
  quantity: string;
  unit_price: string;
  fees: string;
  occurred_at: string;
  notes?: string | null;
}

interface PortfolioSummary {
  positions: Position[];
}

interface Position {
  asset_id: string;
  broker_id: string;
  quantity: string;
}

interface InstrumentSuggestion {
  symbol: string;
  name: string;
  market: string;
  category: string;
  currency: string;
  indicative_price: string;
  source: string;
  as_of_unix: number;
}

interface BrokerList {
  brokers: Broker[];
}

interface AssetList {
  assets: Asset[];
}

interface TransactionList {
  transactions: Transaction[];
}

interface InstrumentSearch {
  items: InstrumentSuggestion[];
  cache: "fresh" | "hit" | "stale" | "miss";
}

const client = new HttpClient();
const routes = {
  brokers: "/api/brokers",
  assets: "/api/assets",
  instrumentSearch: (query: string) => `/api/instruments/search?q=${encodeURIComponent(query)}`,
  transactions: "/api/transactions",
  buyTransaction: "/api/transactions/buy",
  sellTransaction: "/api/transactions/sell",
  portfolioSummary: "/api/portfolio/summary",
};

let instrumentSearchController: AbortController | null = null;
let transactionState: TransactionPageState | null = null;

interface TransactionPageState {
  page: HTMLElement;
  assets: Asset[];
  brokers: Broker[];
  transactions: Transaction[];
  positions: Position[];
}

export function bootPortfolioPages(): void {
  bootBrokersPage();
  bootAssetsPage();
  bootTransactionsPage();
}

function bootBrokersPage(): void {
  const page = document.querySelector<HTMLElement>("[data-brokers-page]");
  if (page === null) return;

  const form = page.querySelector<HTMLFormElement>("[data-broker-form]");
  const cancel = page.querySelector<HTMLButtonElement>("[data-broker-cancel]");
  if (form !== null) {
    bindAccessibleValidation(form);
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void submitBroker(form, page);
    });
  }
  cancel?.addEventListener("click", () => resetBrokerForm(form));

  void loadBrokers(page);
}

async function submitBroker(form: HTMLFormElement, page: HTMLElement): Promise<void> {
  if (!ensureFormValidity(form)) return;
  const name = inputValue(form, "name");
  if (name.length < 2) return showValidationError(form, ["name"], "Informe uma corretora válida.");

  try {
    clearFormErrors(form);
    const id = form.dataset.editingId;
    if (id === undefined) {
      await client.post<{ name: string }, Broker>(routes.brokers, { name });
      showSuccess("Corretora cadastrada");
    } else {
      const version = Number(form.dataset.editingVersion ?? "0");
      await client.patch<{ name: string; version: number }, Broker>(`${routes.brokers}/${id}`, {
        name,
        version,
      });
      showSuccess("Corretora atualizada");
    }
    resetBrokerForm(form);
    await loadBrokers(page);
  } catch (error) {
    showError(error);
  }
}

async function loadBrokers(page: HTMLElement): Promise<Broker[]> {
  const target = page.querySelector<HTMLElement>("[data-broker-list]");
  try {
    const { brokers } = await client.get<BrokerList>(routes.brokers);
    renderBrokers(target, brokers, page);
    return brokers;
  } catch (error) {
    showError(error);
    return [];
  }
}

function renderBrokers(target: HTMLElement | null, brokers: Broker[], page: HTMLElement): void {
  if (target === null) return;
  if (brokers.length === 0) {
    target.replaceChildren(rowWithMessage("Nenhuma corretora cadastrada.", 3));
    return;
  }

  target.replaceChildren(
    ...brokers.map((broker) => {
      const row = document.createElement("tr");
      row.append(
        tableCell(broker.name),
        tableCell(broker.is_archived ? "Arquivada" : "Ativa"),
        actionCell([
          button("Editar", "btn-outline-primary", () => fillBrokerForm(page, broker)),
          button("Arquivar", "btn-outline-danger", () => void archiveBroker(page, broker), broker.is_archived),
        ]),
      );
      return row;
    }),
  );
}

async function archiveBroker(page: HTMLElement, broker: Broker): Promise<void> {
  if (!(await confirmAction(`Arquivar a corretora ${broker.name}?`))) return;

  try {
    await client.post<Record<string, never>, void>(`${routes.brokers}/${broker.id}/archive`, {});
    showSuccess("Corretora arquivada");
    await loadBrokers(page);
  } catch (error) {
    showError(error);
  }
}

function fillBrokerForm(page: HTMLElement, broker: Broker): void {
  const form = page.querySelector<HTMLFormElement>("[data-broker-form]");
  const submit = page.querySelector<HTMLButtonElement>("[data-broker-submit]");
  const cancel = page.querySelector<HTMLButtonElement>("[data-broker-cancel]");
  if (form === null) return;
  setInputValue(form, "name", broker.name);
  form.dataset.editingId = broker.id;
  form.dataset.editingVersion = String(broker.version);
  if (submit !== null) submit.textContent = "Salvar corretora";
  if (cancel !== null) cancel.hidden = false;
}

function resetBrokerForm(form: HTMLFormElement | null): void {
  if (form === null) return;
  form.reset();
  delete form.dataset.editingId;
  delete form.dataset.editingVersion;
  const submit = document.querySelector<HTMLButtonElement>("[data-broker-submit]");
  const cancel = document.querySelector<HTMLButtonElement>("[data-broker-cancel]");
  if (submit !== null) submit.textContent = "Cadastrar corretora";
  if (cancel !== null) cancel.hidden = true;
}

function bootAssetsPage(): void {
  const page = document.querySelector<HTMLElement>("[data-assets-page]");
  if (page === null) return;

  const form = page.querySelector<HTMLFormElement>("[data-asset-form]");
  const cancel = page.querySelector<HTMLButtonElement>("[data-asset-cancel]");
  const symbol = page.querySelector<HTMLInputElement>("#asset-symbol");
  if (form !== null) {
    bindAccessibleValidation(form);
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void submitAsset(form, page);
    });
  }
  cancel?.addEventListener("click", () => resetAssetForm(form));
  symbol?.addEventListener("input", debounce(() => void searchInstrument(page, symbol.value), 350));

  void loadAssets(page);
}

async function submitAsset(form: HTMLFormElement, page: HTMLElement): Promise<void> {
  if (!ensureFormValidity(form)) return;
  const payload = {
    symbol: inputValue(form, "symbol").toUpperCase(),
    name: inputValue(form, "name"),
    market: inputValue(form, "market"),
    category: inputValue(form, "category"),
    currency: inputValue(form, "currency"),
    current_price: parseDecimal(inputValue(form, "current_price") || "0"),
  };

  if (payload.symbol.length < 2 || payload.name.length < 2 || payload.current_price === null) {
    return showValidationError(form, ["symbol", "name"], "Revise símbolo e nome do ativo.");
  }

  try {
    clearFormErrors(form);
    const id = form.dataset.editingId;
    if (id === undefined) {
      await client.post<typeof payload, Asset>(routes.assets, payload);
      showSuccess("Ativo cadastrado", "Para aparecer no dashboard e no extrato, registre uma compra em Movimentações.");
    } else {
      const version = Number(form.dataset.editingVersion ?? "0");
      await client.patch<typeof payload & { version: number }, Asset>(`${routes.assets}/${id}`, {
        ...payload,
        version,
      });
      showSuccess("Ativo atualizado");
    }
    resetAssetForm(form);
    await loadAssets(page);
  } catch (error) {
    showError(error);
  }
}

async function loadAssets(page: HTMLElement): Promise<Asset[]> {
  const target = page.querySelector<HTMLElement>("[data-asset-list]");
  try {
    const { assets } = await client.get<AssetList>(routes.assets);
    renderAssets(target, assets, page);
    return assets;
  } catch (error) {
    showError(error);
    return [];
  }
}

function renderAssets(target: HTMLElement | null, assets: Asset[], page: HTMLElement): void {
  if (target === null) return;
  if (assets.length === 0) {
    target.replaceChildren(rowWithMessage("Nenhum ativo cadastrado.", 5));
    return;
  }

  target.replaceChildren(
    ...assets.map((asset) => {
      const row = document.createElement("tr");
      row.append(
        tableCell(asset.symbol),
        tableCell(asset.name),
        tableCell(asset.market),
        tableCell(asset.currency),
        actionCell([
          linkButton("Comprar", "btn-primary", transactionHref(asset.id, "buy")),
          linkButton("Vender", "btn-outline-primary", transactionHref(asset.id, "sell")),
          button("Editar", "btn-outline-secondary", () => fillAssetForm(page, asset)),
        ]),
      );
      return row;
    }),
  );
}

async function searchInstrument(page: HTMLElement, rawQuery: string): Promise<void> {
  const query = rawQuery.trim().toUpperCase();
  const status = page.querySelector<HTMLElement>("[data-instrument-status]");
  const target = page.querySelector<HTMLElement>("[data-instrument-suggestions]");
  if (target === null) return;

  instrumentSearchController?.abort();
  instrumentSearchController = null;

  if (query.length < 2) {
    target.replaceChildren();
    if (status !== null) status.textContent = "Digite ao menos 2 caracteres para consultar metadados locais.";
    return;
  }

  const controller = new AbortController();
  instrumentSearchController = controller;
  if (status !== null) status.textContent = "Consultando metadados...";

  try {
    const result = await client.get<InstrumentSearch>(routes.instrumentSearch(query), controller.signal);
    if (controller.signal.aborted) return;
    if (status !== null) status.textContent = `Fonte: ${result.cache}. Se não encontrar, preencha manualmente.`;
    target.replaceChildren(
      ...result.items.map((item) => {
        const option = document.createElement("button");
        option.type = "button";
        option.className = "list-group-item list-group-item-action";
        option.textContent = `${item.symbol} — ${item.name} (${item.market}/${item.currency}) · ${formatTimestamp(item.as_of_unix)}`;
        option.addEventListener("click", () => fillAssetSuggestion(page, item));
        return option;
      }),
    );
    if (result.items.length === 0) {
      target.replaceChildren();
      if (status !== null) status.textContent = "Nenhum metadado encontrado. Preencha manualmente.";
    }
  } catch (error) {
    if (controller.signal.aborted) return;
    showError(error);
  } finally {
    if (instrumentSearchController === controller) instrumentSearchController = null;
  }
}

function fillAssetSuggestion(page: HTMLElement, item: InstrumentSuggestion): void {
  const form = page.querySelector<HTMLFormElement>("[data-asset-form]");
  if (form === null) return;
  setInputValue(form, "symbol", item.symbol);
  setInputValue(form, "name", item.name);
  setInputValue(form, "market", item.market);
  setInputValue(form, "category", item.category);
  setInputValue(form, "currency", item.currency);
  setInputValue(form, "current_price", item.indicative_price);
  page.querySelector<HTMLElement>("[data-instrument-suggestions]")?.replaceChildren();
  const status = page.querySelector<HTMLElement>("[data-instrument-status]");
  if (status !== null) status.textContent = `Metadados aplicados de ${item.source}, atualizados em ${formatTimestamp(item.as_of_unix)}.`;
}

function fillAssetForm(page: HTMLElement, asset: Asset): void {
  const form = page.querySelector<HTMLFormElement>("[data-asset-form]");
  const submit = page.querySelector<HTMLButtonElement>("[data-asset-submit]");
  const cancel = page.querySelector<HTMLButtonElement>("[data-asset-cancel]");
  if (form === null) return;
  setInputValue(form, "symbol", asset.symbol);
  setInputValue(form, "name", asset.name);
  setInputValue(form, "market", asset.market);
  setInputValue(form, "category", asset.category);
  setInputValue(form, "currency", asset.currency);
  setInputValue(form, "current_price", asset.current_price);
  form.dataset.editingId = asset.id;
  form.dataset.editingVersion = String(asset.version);
  if (submit !== null) submit.textContent = "Salvar ativo";
  if (cancel !== null) cancel.hidden = false;
}

function resetAssetForm(form: HTMLFormElement | null): void {
  if (form === null) return;
  form.reset();
  delete form.dataset.editingId;
  delete form.dataset.editingVersion;
  document.querySelector<HTMLElement>("[data-instrument-suggestions]")?.replaceChildren();
  const submit = document.querySelector<HTMLButtonElement>("[data-asset-submit]");
  const cancel = document.querySelector<HTMLButtonElement>("[data-asset-cancel]");
  if (submit !== null) submit.textContent = "Cadastrar ativo";
  if (cancel !== null) cancel.hidden = true;
}

function bootTransactionsPage(): void {
  const page = document.querySelector<HTMLElement>("[data-transactions-page]");
  if (page === null) return;

  const form = page.querySelector<HTMLFormElement>("[data-transaction-form]");
  if (form !== null) {
    bindAccessibleValidation(form);
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void submitTransaction(form, page);
    });
    form.addEventListener("change", () => updateAvailablePosition(page));
  }
  page.querySelector<HTMLFormElement>("[data-transaction-filters]")?.addEventListener("input", () => applyTransactionFilters());
  page.querySelector<HTMLFormElement>("[data-transaction-filters]")?.addEventListener("change", () => applyTransactionFilters());

  void refreshTransactionsPage(page);
}

async function refreshTransactionsPage(page: HTMLElement): Promise<void> {
  const [assets, brokers, summary] = await Promise.all([loadAssetOptions(page), loadBrokerOptions(page), client.get<PortfolioSummary>(routes.portfolioSummary)]);
  applyTransactionPrefill(page);
  await loadTransactions(page, assets, brokers, summary.positions);
  updateAvailablePosition(page);
}

async function loadAssetOptions(page: HTMLElement): Promise<Asset[]> {
  const select = page.querySelector<HTMLSelectElement>("[data-asset-select]");
  const { assets } = await client.get<AssetList>(routes.assets);
  if (select !== null) {
    select.disabled = assets.length === 0;
    select.replaceChildren(
      ...(assets.length === 0
        ? [option("", "Cadastre um ativo primeiro")]
        : assets.map((asset) => option(asset.id, `${asset.symbol} — ${asset.name}`))),
    );
  }
  const filterSelect = page.querySelector<HTMLSelectElement>("[data-filter-asset]");
  if (filterSelect !== null) {
    filterSelect.replaceChildren(option("", "Todos os ativos"), ...assets.map((asset) => option(asset.id, `${asset.symbol} — ${asset.name}`)));
  }
  return assets;
}

async function loadBrokerOptions(page: HTMLElement): Promise<Broker[]> {
  const select = page.querySelector<HTMLSelectElement>("[data-broker-select]");
  const { brokers } = await client.get<BrokerList>(routes.brokers);
  const active = brokers.filter((broker) => !broker.is_archived);
  if (select !== null) {
    select.disabled = active.length === 0;
    select.replaceChildren(
      ...(active.length === 0
        ? [option("", "Cadastre uma corretora ativa primeiro")]
        : active.map((broker) => option(broker.id, broker.name))),
    );
  }
  return brokers;
}

async function submitTransaction(form: HTMLFormElement, page: HTMLElement): Promise<void> {
  if (!ensureFormValidity(form)) return;
  const type = inputValue(form, "transaction_type");
  const payload = {
    asset_id: inputValue(form, "asset_id"),
    broker_id: inputValue(form, "broker_id"),
    quantity: parseDecimal(inputValue(form, "quantity")),
    unit_price: parseDecimal(inputValue(form, "unit_price")),
    fees: parseDecimal(inputValue(form, "fees") || "0"),
  };
  const date = inputValue(form, "occurred_at");
  const notes = inputValue(form, "notes");

  if (payload.asset_id === "" || payload.broker_id === "" || payload.quantity === null || payload.unit_price === null || payload.fees === null) {
    return showValidationError(form, ["asset_id", "broker_id", "quantity", "unit_price", "fees"], "Revise ativo, corretora, quantidade, preço e taxas.");
  }

  const request: {
    asset_id: string;
    broker_id: string;
    quantity: string;
    unit_price: string;
    fees: string;
    occurred_at_unix?: number;
    notes?: string;
  } = {
    asset_id: payload.asset_id,
    broker_id: payload.broker_id,
    quantity: payload.quantity,
    unit_price: payload.unit_price,
    fees: payload.fees,
  };
  if (date !== "") request.occurred_at_unix = Math.floor(new Date(`${date}T12:00:00`).getTime() / 1000);
  if (notes !== "") request.notes = notes;

  try {
    clearFormErrors(form);
    await client.post<typeof request, Transaction>(type === "sell" ? routes.sellTransaction : routes.buyTransaction, request);
    showSuccess("Movimentação registrada");
    form.reset();
    await refreshTransactionsPage(page);
  } catch (error) {
    showError(error);
  }
}

async function loadTransactions(page: HTMLElement, assets: Asset[], brokers: Broker[], positions: Position[]): Promise<void> {
  const target = page.querySelector<HTMLElement>("[data-transaction-list]");
  try {
    const { transactions } = await client.get<TransactionList>(routes.transactions);
    transactionState = { page, assets, brokers, transactions, positions };
    renderTransactions(target, transactions, assets, brokers);
  } catch (error) {
    showError(error);
  }
}

function applyTransactionFilters(): void {
  if (transactionState === null) return;
  const { page, assets, brokers, transactions } = transactionState;
  const target = page.querySelector<HTMLElement>("[data-transaction-list]");
  const form = page.querySelector<HTMLFormElement>("[data-transaction-filters]");
  if (form === null) {
    renderTransactions(target, transactions, assets, brokers);
    return;
  }

  const assetId = inputValue(form, "asset_id");
  const type = inputValue(form, "transaction_type");
  const from = inputValue(form, "from");
  const to = inputValue(form, "to");
  const fromTime = from === "" ? null : new Date(`${from}T00:00:00`).getTime();
  const toTime = to === "" ? null : new Date(`${to}T23:59:59`).getTime();

  const filtered = transactions.filter((transaction) => {
    const transactionTime = new Date(transaction.occurred_at).getTime();
    if (assetId !== "" && transaction.asset_id !== assetId) return false;
    if (type !== "" && transaction.transaction_type !== type) return false;
    if (fromTime !== null && transactionTime < fromTime) return false;
    if (toTime !== null && transactionTime > toTime) return false;
    return true;
  });

  renderTransactions(target, filtered, assets, brokers);
}

function renderTransactions(target: HTMLElement | null, transactions: Transaction[], assets: Asset[], brokers: Broker[]): void {
  if (target === null) return;
  if (transactions.length === 0) {
    target.replaceChildren(rowWithMessage("Nenhuma movimentação encontrada. Cadastre uma compra para o ativo aparecer no dashboard e no extrato.", 7));
    return;
  }

  const assetNames = new Map(assets.map((asset) => [asset.id, asset.symbol]));
  const assetCurrencies = new Map(assets.map((asset) => [asset.id, asset.currency]));
  const brokerNames = new Map(brokers.map((broker) => [broker.id, broker.name]));
  target.replaceChildren(
    ...transactions.map((transaction) => {
      const row = document.createElement("tr");
      const currency = assetCurrencies.get(transaction.asset_id) ?? "BRL";
      row.append(
        tableCell(formatDate(transaction.occurred_at)),
        tableCell(transaction.transaction_type === "BUY" ? "Compra" : "Venda"),
        tableCell(assetNames.get(transaction.asset_id) ?? shortId(transaction.asset_id)),
        tableCell(brokerNames.get(transaction.broker_id) ?? shortId(transaction.broker_id)),
        tableCell(formatDecimal(transaction.quantity), "text-end"),
        tableCell(money(transaction.unit_price, currency), "text-end"),
        tableCell(money(transaction.fees, currency), "text-end"),
      );
      return row;
    }),
  );
}

function updateAvailablePosition(page: HTMLElement): void {
  const target = page.querySelector<HTMLElement>("[data-available-position]");
  const form = page.querySelector<HTMLFormElement>("[data-transaction-form]");
  if (target === null || form === null || transactionState === null) return;

  const assetId = inputValue(form, "asset_id");
  const brokerId = inputValue(form, "broker_id");
  const type = inputValue(form, "transaction_type");
  if (assetId === "" || brokerId === "") {
    target.textContent = "Selecione ativo e corretora para ver a posição disponível.";
    return;
  }

  const quantity = transactionState.positions.find((position) => position.asset_id === assetId && position.broker_id === brokerId)?.quantity ?? "0";
  target.textContent = type === "sell"
    ? `Disponível para venda nesta corretora: ${quantity}`
    : `Posição atual nesta corretora: ${quantity}`;
}

function inputValue(form: HTMLFormElement, name: string): string {
  const element = form.elements.namedItem(name);
  if (element instanceof HTMLInputElement || element instanceof HTMLSelectElement || element instanceof HTMLTextAreaElement) {
    return element.value.trim();
  }
  return "";
}

function setInputValue(form: HTMLFormElement, name: string, value: string): void {
  const element = form.elements.namedItem(name);
  if (element instanceof HTMLInputElement || element instanceof HTMLSelectElement || element instanceof HTMLTextAreaElement) {
    element.value = value;
  }
}

function parseDecimal(value: string): string | null {
  const normalized = value.trim().replace(",", ".");
  return /^\d+(\.\d{1,8})?$/.test(normalized) ? normalized : null;
}

function tableCell(value: string, className?: string): HTMLTableCellElement {
  const cell = document.createElement("td");
  cell.textContent = value;
  if (className !== undefined) cell.className = className;
  return cell;
}

function actionCell(actions: HTMLElement[]): HTMLTableCellElement {
  const cell = tableCell("", "text-end");
  const group = document.createElement("div");
  group.className = "btn-group btn-group-sm";
  group.append(...actions);
  cell.append(group);
  return cell;
}

function linkButton(label: string, variant: string, href: string): HTMLAnchorElement {
  const element = document.createElement("a");
  element.className = `btn ${variant}`;
  element.href = href;
  element.textContent = label;
  return element;
}

function button(label: string, variant: string, onClick: () => void, disabled = false): HTMLButtonElement {
  const element = document.createElement("button");
  element.type = "button";
  element.className = `btn ${variant}`;
  element.textContent = label;
  element.disabled = disabled;
  element.addEventListener("click", onClick);
  return element;
}

function rowWithMessage(message: string, columns: number): HTMLTableRowElement {
  const row = document.createElement("tr");
  const cell = tableCell(message);
  cell.colSpan = columns;
  row.append(cell);
  return row;
}

function option(value: string, label: string): HTMLOptionElement {
  const element = document.createElement("option");
  element.value = value;
  element.textContent = label;
  return element;
}

function money(value: string, currency: string): string {
  return new Intl.NumberFormat("pt-BR", { style: "currency", currency }).format(Number(value));
}

function formatDecimal(value: string): string {
  return new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 8 }).format(Number(value));
}

function formatDate(value: string): string {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleDateString("pt-BR");
}

function formatTimestamp(value: number): string {
  const parsed = new Date(value * 1000);
  return Number.isNaN(parsed.getTime()) ? "data indisponível" : parsed.toLocaleString("pt-BR");
}

function shortId(value: string): string {
  return value.slice(0, 8);
}

function transactionHref(assetId: string, type: "buy" | "sell"): string {
  const params = new URLSearchParams({ asset_id: assetId, type });
  return `/transactions?${params.toString()}`;
}

function applyTransactionPrefill(page: HTMLElement): void {
  const params = new URLSearchParams(window.location.search);
  const assetId = params.get("asset_id");
  const type = params.get("type");
  const form = page.querySelector<HTMLFormElement>("[data-transaction-form]");
  if (form === null) return;

  if (type === "buy" || type === "sell") setInputValue(form, "transaction_type", type);
  if (assetId !== null) setInputValue(form, "asset_id", assetId);
}

function showValidationError(form: HTMLFormElement, fieldNames: string[], message: string): void {
  showFormError(form, fieldNames, message);
  showError({ code: "validation_error", message, status: 400 });
}

function debounce(callback: () => void, waitMs: number): () => void {
  let handle = 0;
  return () => {
    window.clearTimeout(handle);
    handle = window.setTimeout(callback, waitMs);
  };
}
