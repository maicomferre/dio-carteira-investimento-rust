type FormField = HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;

export function bindAccessibleValidation(form: HTMLFormElement): void {
  form.addEventListener("invalid", (event) => {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement || event.target instanceof HTMLTextAreaElement) {
      markFieldError(event.target, event.target.validationMessage);
      focusFirstInvalid(form);
    }
  }, true);

  form.addEventListener("input", (event) => {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
      clearFieldError(event.target);
    }
  });

  form.addEventListener("change", (event) => {
    if (event.target instanceof HTMLSelectElement) {
      clearFieldError(event.target);
    }
  });
}

export function ensureFormValidity(form: HTMLFormElement): boolean {
  clearFormErrors(form);
  if (form.checkValidity()) return true;

  const firstInvalid = firstInvalidField(form);
  if (firstInvalid !== null) {
    markFieldError(firstInvalid, firstInvalid.validationMessage);
    announceFormError(form, "Revise o campo destacado antes de continuar.");
    firstInvalid.focus();
  }
  form.reportValidity();
  return false;
}

export function showFormError(form: HTMLFormElement, fieldNames: string[], message: string): void {
  clearFormErrors(form);
  const fields = fieldNames
    .map((name) => form.elements.namedItem(name))
    .filter((field): field is FormField => field instanceof HTMLInputElement || field instanceof HTMLSelectElement || field instanceof HTMLTextAreaElement);

  fields.forEach((field) => markFieldError(field, message));
  announceFormError(form, message);
  fields[0]?.focus();
}

export function clearFormErrors(form: HTMLFormElement): void {
  form.querySelectorAll<FormField>(".is-invalid").forEach(clearFieldError);
  const status = form.querySelector<HTMLElement>("[data-form-status]");
  if (status !== null) {
    status.textContent = "";
    status.hidden = true;
  }
}

function markFieldError(field: FormField, message: string): void {
  field.classList.add("is-invalid");
  field.setAttribute("aria-invalid", "true");
  const feedback = feedbackFor(field);
  if (feedback !== null) {
    feedback.textContent = message;
    feedback.hidden = false;
  }
}

function clearFieldError(field: FormField): void {
  field.classList.remove("is-invalid");
  field.removeAttribute("aria-invalid");
  const feedback = feedbackFor(field);
  if (feedback !== null) {
    feedback.textContent = "";
    feedback.hidden = true;
  }
}

function feedbackFor(field: FormField): HTMLElement | null {
  if (field.id === "") return null;
  return document.querySelector<HTMLElement>(`[data-error-for="${field.id}"]`);
}

function announceFormError(form: HTMLFormElement, message: string): void {
  const status = form.querySelector<HTMLElement>("[data-form-status]");
  if (status === null) return;
  status.textContent = message;
  status.hidden = false;
  status.focus();
}

function firstInvalidField(form: HTMLFormElement): FormField | null {
  return form.querySelector<FormField>("input:invalid, select:invalid, textarea:invalid");
}

function focusFirstInvalid(form: HTMLFormElement): void {
  firstInvalidField(form)?.focus();
}
