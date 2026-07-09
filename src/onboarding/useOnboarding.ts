import { useState } from "react";
import { ONBOARDING_STEPS } from "./onboarding-steps";

export function useOnboarding(): {
  step: number;
  next: () => void;
  prev: () => void;
  isLast: boolean;
} {
  const [step, setStep] = useState(0);
  const isLast = step === ONBOARDING_STEPS.length - 1;

  return {
    step,
    next: () => setStep((s) => Math.min(s + 1, ONBOARDING_STEPS.length - 1)),
    prev: () => setStep((s) => Math.max(s - 1, 0)),
    isLast,
  };
}
