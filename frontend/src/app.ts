import { showError, showSuccess } from "./alerts.js";
import { bootDashboard } from "./dashboard.js";
import { bindAccessibleValidation, clearFormErrors, ensureFormValidity, showFormError } from "./forms.js";
import { HttpClient } from "./http.js";
import { bootPortfolioPages } from "./portfolio-pages.js";

const client = new HttpClient();

document.addEventListener("DOMContentLoaded", () => {
  bindAuthForms();
  bindLogout();
  bootDashboard();
  bootPortfolioPages();
});

function bindAuthForms(): void {
  document.querySelectorAll<HTMLFormElement>("[data-auth-form]").forEach((form) => {
    bindAccessibleValidation(form);
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void submitAuthForm(form);
    });
  });
}

async function submitAuthForm(form: HTMLFormElement): Promise<void> {
  const username = form.elements.namedItem("username");
  const password = form.elements.namedItem("password");
  if (!(username instanceof HTMLInputElement) || !(password instanceof HTMLInputElement)) return;
  if (!ensureFormValidity(form)) return;

  const endpoint = form.dataset.authForm;
  if (endpoint !== "/auth/login" && endpoint !== "/auth/register") return;

  try {
    clearFormErrors(form);
    if (endpoint === "/auth/register") {
      await client.post(endpoint, { username: username.value, password: password.value });
      showSuccess("Cadastro criado");
    }

    await client.post("/auth/login", { username: username.value, password: password.value });
    window.location.assign("/dashboard");
  } catch (error) {
    password.value = "";
    showFormError(form, ["username", "password"], "Não foi possível autenticar com os dados informados.");
    showError(error);
  }
}

function bindLogout(): void {
  document.querySelectorAll<HTMLFormElement>("[data-logout-form]").forEach((form) => {
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void submitLogout();
    });
  });
}

async function submitLogout(): Promise<void> {
  try {
    await client.post<Record<string, never>, void>("/auth/logout", {});
    window.location.assign("/login");
  } catch (error) {
    showError(error);
  }
}
