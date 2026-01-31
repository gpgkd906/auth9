import { createRoot } from "react-dom/client";
import { StrictMode, lazy, Suspense } from "react";
import { KcPage } from "./kc.gen";

// Lazy load the app entrypoint for development
const AppEntrypoint = lazy(() => import("./main.app"));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {window.kcContext ? (
      <KcPage kcContext={window.kcContext} />
    ) : (
      <Suspense fallback={<div>Loading...</div>}>
        <AppEntrypoint />
      </Suspense>
    )}
  </StrictMode>
);
