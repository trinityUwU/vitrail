import type { ReactElement } from "react";
import { Fragment } from "react";
import { CheckCircle, XCircle } from "lucide-react";
import { Button } from "../shared/components/Button";
import { useOnboarding } from "./useOnboarding";
import { ONBOARDING_STEPS } from "./onboarding-steps";
import "./Onboarding.css";

export function Onboarding({ onDone }: { onDone: () => void }): ReactElement {
  const { step, next, prev, isLast } = useOnboarding();
  const current = ONBOARDING_STEPS[step];

  return (
    <div>
      <div className="onboard-progress">
        {ONBOARDING_STEPS.map((_, i) => (
          <Fragment key={`step-${i}`}>
            <div className={`onboard-step-dot ${i < step ? "done" : i === step ? "current" : ""}`}>
              {i < step ? "✓" : i + 1}
            </div>
            {i < ONBOARDING_STEPS.length - 1 && (
              <div className={`onboard-step-line${i < step ? " done" : ""}`} />
            )}
          </Fragment>
        ))}
      </div>
      <div className="onboard-card">
        <h2>{current.title}</h2>
        <p>{current.description}</p>
        {current.checks.map((c) => (
          <div className={`onboard-check ${c.ok ? "ok" : "fail"}`} key={c.label}>
            {c.ok ? <CheckCircle style={{ color: "var(--ok)" }} /> : <XCircle style={{ color: "var(--danger)" }} />}
            <span>{c.label}</span>
          </div>
        ))}
        <div style={{ marginTop: 28 }}>
          {step > 0 && !isLast && (
            <Button variant="ghost" size="lg" style={{ marginRight: 12 }} onClick={prev}>
              Retour
            </Button>
          )}
          <Button variant="primary" size="lg" onClick={isLast ? onDone : next}>
            {isLast ? "Accéder au tableau de bord" : "Continuer"}
          </Button>
        </div>
      </div>
    </div>
  );
}
