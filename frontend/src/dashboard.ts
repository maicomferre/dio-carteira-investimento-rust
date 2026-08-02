import { showError } from "./alerts.js";
import { HttpClient } from "./http.js";

interface PortfolioSummary {
  positions: Position[];
  totals_by_currency: MoneyTotal[];
  allocation_by_category: CategoryAllocation[];
  allocation_by_broker: BrokerAllocation[];
  daily_cash_flow: DailyCashFlow[];
}

interface Position {
  asset_id: string;
  broker_id: string;
  currency: string;
  category: string;
  quantity: string;
  cost_basis: string;
  average_cost: string;
}

interface MoneyTotal {
  currency: string;
  total: string;
}

interface CategoryAllocation {
  currency: string;
  category: string;
  total: string;
}

interface BrokerAllocation {
  currency: string;
  broker_id: string;
  total: string;
}

interface DailyCashFlow {
  date: string;
  purchases: string;
  sales: string;
  fees: string;
  net_flow: string;
}

interface Broker {
  id: string;
  name: string;
}

interface Asset {
  id: string;
  symbol: string;
  name: string;
  currency: string;
}

const client = new HttpClient();

export function bootDashboard(): void {
  const root = document.querySelector<HTMLElement>("[data-dashboard]");
  if (root === null) return;

  void loadDashboard(root);
}

async function loadDashboard(root: HTMLElement): Promise<void> {
  try {
    const [summary, assets, brokers] = await Promise.all([
      client.get<PortfolioSummary>("/api/portfolio/summary"),
      client.get<{ assets: Asset[] }>("/api/assets"),
      client.get<{ brokers: Broker[] }>("/api/brokers"),
    ]);
    const labels = {
      assets: new Map(assets.assets.map((asset) => [asset.id, asset.symbol])),
      brokers: new Map(brokers.brokers.map((broker) => [broker.id, broker.name])),
    };
    renderCurrencyTabs(root, summary, labels);
    const currency = summary.totals_by_currency[0]?.currency ?? "BRL";
    renderCurrency(root, summary, currency, labels);
    renderAssetsWithoutPosition(root, assets.assets, summary.positions);
  } catch (error) {
    showError(error);
  }
}

function renderCurrencyTabs(
  root: HTMLElement,
  summary: PortfolioSummary,
  labels: { assets: Map<string, string>; brokers: Map<string, string> },
): void {
  const target = root.querySelector<HTMLElement>("[data-currency-tabs]");
  if (target === null) return;

  target.replaceChildren(
    ...summary.totals_by_currency.map((item, index) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `btn btn-sm ${index === 0 ? "btn-primary" : "btn-outline-primary"}`;
      button.textContent = item.currency;
      button.addEventListener("click", () => {
        target.querySelectorAll("button").forEach((element) => {
          element.className = "btn btn-sm btn-outline-primary";
        });
        button.className = "btn btn-sm btn-primary";
        renderCurrency(root, summary, item.currency, labels);
      });
      return button;
    }),
  );
}

function renderCurrency(
  root: HTMLElement,
  summary: PortfolioSummary,
  currency: string,
  labels: { assets: Map<string, string>; brokers: Map<string, string> },
): void {
  const total = summary.totals_by_currency.find((item) => item.currency === currency);
  setText(root, "[data-total-selected]", money(total?.total ?? "0", currency));
  renderDonut(root, "[data-category-chart]", summary.allocation_by_category.filter((item) => item.currency === currency));
  renderBars(root, summary.positions.filter((item) => item.currency === currency), currency, labels);
  renderBrokerDonut(root, summary.allocation_by_broker.filter((item) => item.currency === currency), labels.brokers);
  renderCashFlow(root, summary.daily_cash_flow);
}

function renderDonut(root: HTMLElement, selector: string, items: Array<{ category: string; total: string }>): void {
  const target = root.querySelector<HTMLElement>(selector);
  const fallback = root.querySelector<HTMLElement>("[data-category-fallback]");
  if (target === null) return;
  if (items.length === 0) {
    target.textContent = "Sem posições para exibir.";
    fallback?.replaceChildren();
    return;
  }

  target.replaceChildren(svgDonut(items.map((item) => ({ label: item.category, value: Number(item.total) }))));
  renderFallbackList(fallback, items.map((item) => ({ label: item.category, value: item.total })));
}

function renderBrokerDonut(root: HTMLElement, items: BrokerAllocation[], brokerLabels: Map<string, string>): void {
  const section = root.querySelector<HTMLElement>("[data-broker-section]");
  const target = root.querySelector<HTMLElement>("[data-broker-chart]");
  if (section === null || target === null) return;

  if (items.filter((item) => Number(item.total) > 0).length < 2) {
    section.hidden = true;
    return;
  }

  section.hidden = false;
  target.replaceChildren(svgDonut(items.map((item) => ({ label: brokerLabels.get(item.broker_id) ?? shortId(item.broker_id), value: Number(item.total) }))));
  renderFallbackList(
    root.querySelector<HTMLElement>("[data-broker-fallback]"),
    items.map((item) => ({ label: brokerLabels.get(item.broker_id) ?? shortId(item.broker_id), value: item.total })),
  );
}

function renderBars(
  root: HTMLElement,
  positions: Position[],
  currency: string,
  labels: { assets: Map<string, string>; brokers: Map<string, string> },
): void {
  const target = root.querySelector<HTMLElement>("[data-asset-bars]");
  if (target === null) return;
  if (positions.length === 0) {
    target.textContent = "Sem ativos nesta moeda.";
    return;
  }

  const max = Math.max(...positions.map((item) => Number(item.cost_basis)));
  target.replaceChildren(
    ...positions.map((item) => {
      const wrapper = document.createElement("div");
      wrapper.className = "mb-3";
      const label = document.createElement("div");
      label.className = "d-flex justify-content-between small";
      const asset = labels.assets.get(item.asset_id) ?? shortId(item.asset_id);
      const broker = labels.brokers.get(item.broker_id) ?? shortId(item.broker_id);
      label.append(textSpan(`${asset} · ${broker}`), textSpan(money(item.cost_basis, currency)));
      const progress = document.createElement("progress");
      progress.className = "w-100";
      progress.max = 100;
      progress.value = max === 0 ? 0 : (Number(item.cost_basis) / max) * 100;
      progress.setAttribute("aria-label", `${asset} em ${broker}`);
      wrapper.append(label, progress);
      return wrapper;
    }),
  );
}

function renderCashFlow(root: HTMLElement, flows: DailyCashFlow[]): void {
  const target = root.querySelector<HTMLElement>("[data-cash-flow]");
  if (target === null) return;
  if (flows.length === 0) {
    target.textContent = "Sem movimentações registradas.";
    return;
  }

  const latest = flows.slice(-7);
  target.replaceChildren(
    ...latest.map((item) => {
      const row = document.createElement("li");
      row.className = "list-group-item d-flex justify-content-between";
      row.append(textSpan(item.date), textSpan(`líquido ${item.net_flow}`));
      return row;
    }),
  );
}

function renderAssetsWithoutPosition(root: HTMLElement, assets: Asset[], positions: Position[]): void {
  const section = root.querySelector<HTMLElement>("[data-assets-without-position-section]");
  const target = root.querySelector<HTMLElement>("[data-assets-without-position]");
  if (section === null || target === null) return;

  const positionedAssetIds = new Set(positions.map((position) => position.asset_id));
  const pendingAssets = assets.filter((asset) => !positionedAssetIds.has(asset.id));

  if (pendingAssets.length === 0) {
    section.hidden = true;
    target.replaceChildren();
    return;
  }

  section.hidden = false;
  target.replaceChildren(
    ...pendingAssets.map((asset) => {
      const item = document.createElement("li");
      item.className = "list-group-item d-flex justify-content-between align-items-center";
      item.append(textSpan(`${asset.symbol} — ${asset.name}`), badge(`${asset.currency} · sem compra`));
      return item;
    }),
  );
}

function svgDonut(items: Array<{ label: string; value: number }>): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 42 42");
  svg.setAttribute("role", "img");
  svg.setAttribute("aria-label", donutLabel(items));
  svg.setAttribute("class", "dashboard-donut");
  const total = items.reduce((sum, item) => sum + item.value, 0);
  let offset = 25;
  const colors = ["#0d6efd", "#198754", "#ffc107", "#dc3545", "#6f42c1", "#0dcaf0"];

  items.forEach((item, index) => {
    if (item.value <= 0 || total <= 0) return;
    const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    const percent = (item.value / total) * 100;
    circle.setAttribute("cx", "21");
    circle.setAttribute("cy", "21");
    circle.setAttribute("r", "15.915");
    circle.setAttribute("fill", "transparent");
    circle.setAttribute("stroke", colors[index % colors.length] ?? "#0d6efd");
    circle.setAttribute("stroke-width", "7");
    circle.setAttribute("stroke-dasharray", `${percent} ${100 - percent}`);
    circle.setAttribute("stroke-dashoffset", String(offset));
    svg.append(circle);
    offset -= percent;
  });

  return svg;
}

function renderFallbackList(target: HTMLElement | null, items: Array<{ label: string; value: string }>): void {
  if (target === null) return;
  const total = items.reduce((sum, item) => sum + Number(item.value), 0);
  target.replaceChildren(
    ...items
      .filter((item) => Number(item.value) > 0)
      .map((item) => {
        const percent = total === 0 ? 0 : (Number(item.value) / total) * 100;
        const row = document.createElement("li");
        row.className = "list-group-item d-flex justify-content-between px-0";
        row.append(textSpan(item.label), textSpan(`${percent.toFixed(1)}%`));
        return row;
      }),
  );
}

function donutLabel(items: Array<{ label: string; value: number }>): string {
  const total = items.reduce((sum, item) => sum + item.value, 0);
  if (total <= 0) return "Gráfico sem dados";
  return items
    .filter((item) => item.value > 0)
    .map((item) => `${item.label}: ${((item.value / total) * 100).toFixed(1)}%`)
    .join("; ");
}

function money(value: string, currency: string): string {
  return new Intl.NumberFormat("pt-BR", { style: "currency", currency }).format(Number(value));
}

function setText(root: HTMLElement, selector: string, value: string): void {
  const target = root.querySelector<HTMLElement>(selector);
  if (target !== null) target.textContent = value;
}

function textSpan(value: string): HTMLSpanElement {
  const span = document.createElement("span");
  span.textContent = value;
  return span;
}

function badge(value: string): HTMLSpanElement {
  const span = document.createElement("span");
  span.className = "badge text-bg-light";
  span.textContent = value;
  return span;
}

function shortId(value: string): string {
  return value.slice(0, 8);
}
