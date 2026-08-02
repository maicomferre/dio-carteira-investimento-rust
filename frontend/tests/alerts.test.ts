import assert from "node:assert/strict";
import { afterEach, beforeEach, test } from "node:test";

import { confirmAction, showError, showSuccess } from "../src/alerts.js";

type AlertCall = {
  title: string;
  text?: string;
  icon?: string;
};

let alertCalls: AlertCall[] = [];
let liveRegionText = "";
let assignedLocation = "";

beforeEach(() => {
  alertCalls = [];
  liveRegionText = "";
  assignedLocation = "";
});

afterEach(() => {
  deleteGlobal("window");
  deleteGlobal("document");
});

test("showSuccess delegates to SweetAlert2 with success icon", () => {
  setWindowWithSwal();

  showSuccess("Ativo cadastrado");

  assert.equal(alertCalls.length, 1);
  assert.equal(alertCalls[0]?.title, "Ativo cadastrado");
  assert.equal(alertCalls[0]?.icon, "success");
});

test("showError includes request id and redirects unauthorized sessions", () => {
  setWindowWithSwal();

  showError({ code: "unauthorized", message: "Faça login novamente.", requestId: "req-1", status: 401 });

  assert.equal(alertCalls[0]?.title, "Sessão expirada");
  assert.equal(alertCalls[0]?.text, "Faça login novamente. ID: req-1");
  assert.equal(assignedLocation, "/login");
});

test("confirmAction resolves true only when SweetAlert2 confirms", async () => {
  setWindowWithSwal(true);

  assert.equal(await confirmAction("Arquivar corretora?"), true);
});

test("confirmAction returns false when SweetAlert2 is unavailable", async () => {
  setWindowWithoutSwal();

  assert.equal(await confirmAction("Arquivar corretora?"), false);
});

test("alerts fall back to live region when SweetAlert2 is unavailable", () => {
  setWindowWithoutSwal();
  setDocumentLiveRegion();

  showError({ code: "validation_error", message: "Revise o campo.", status: 422 });

  assert.equal(liveRegionText, "Dados inválidos. Revise o campo.");
});

function setWindowWithSwal(confirm = false): void {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      location: {
        assign: (url: string) => {
          assignedLocation = url;
        },
      },
      setTimeout: (callback: () => void) => {
        callback();
        return 1;
      },
      Swal: {
        fire: async (options: AlertCall) => {
          alertCalls.push(options);
          return { isConfirmed: confirm };
        },
      },
    },
  });
}

function setWindowWithoutSwal(): void {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {},
  });
}

function setDocumentLiveRegion(): void {
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: {
      querySelector: () => ({
        set textContent(value: string) {
          liveRegionText = value;
        },
      }),
    },
  });
}

function deleteGlobal(name: "document" | "window"): void {
  Reflect.deleteProperty(globalThis, name);
}
