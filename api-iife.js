if ("__TAURI__" in window) {
  var __TAURI_PLUGIN_SHARE__ = (function (n) {
    "use strict";
    async function a(n, a = {}, e) {
      return window.__TAURI_INTERNALS__.invoke(n, a, e);
    }
    return (
      "function" == typeof SuppressedError && SuppressedError,
      (n.cleanup = async function () {
        await a("plugin:share|cleanup");
      }),
      (n.shareData = async function (n) {
        await a("plugin:share|share_data", { options: n });
      }),
      (n.shareFile = async function (n) {
        await a("plugin:share|share_file", { options: n });
      }),
      (n.shareText = async function (n) {
        await a("plugin:share|share_text", { options: n });
      }),
      n
    );
  })({});
  Object.defineProperty(window.__TAURI__, "share", {
    value: __TAURI_PLUGIN_SHARE__,
  });
}
