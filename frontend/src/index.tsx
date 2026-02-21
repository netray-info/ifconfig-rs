/* @refresh reload */
import { render } from "solid-js/web";
import { ErrorBoundary } from "solid-js";
import App from "./App";
import "./styles/global.css";

const root = document.getElementById("root");
if (root) {
  render(
    () => (
      <ErrorBoundary
        fallback={(err) => (
          <div class="container">
            <div class="error-msg">
              Something went wrong. Please reload the page.
              <br />
              <small>{err?.message ?? String(err)}</small>
            </div>
          </div>
        )}
      >
        <App />
      </ErrorBoundary>
    ),
    root,
  );
}
