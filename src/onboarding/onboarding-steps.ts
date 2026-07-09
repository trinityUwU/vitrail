export interface OnboardCheck {
  label: string;
  ok: boolean;
}

export interface OnboardStep {
  title: string;
  description: string;
  checks: OnboardCheck[];
}

// EPICs 1/4/7 remplaceront ces vérifications figées par de vrais contrôles système
// (daemon OpenSnitch, génération CA, élévation nftables, dry-run activation/désactivation).
export const ONBOARDING_STEPS: OnboardStep[] = [
  {
    title: "Vérification d'OpenSnitch",
    description:
      "Vitrail s'appuie sur OpenSnitch pour attribuer chaque flux réseau à son processus source. Nous vérifions que le daemon est bien en cours d'exécution.",
    checks: [
      { label: "Daemon OpenSnitch détecté et actif", ok: true },
      { label: "Version OpenSnitch compatible", ok: true },
    ],
  },
  {
    title: "Certificat d'autorité locale",
    description:
      "Pour déchiffrer le trafic TLS, Vitrail génère une autorité de certification locale. Cette CA sera installée dans votre trust store système. Rien n'est jamais partagé en dehors de cette machine.",
    checks: [
      { label: "CA RSA 4096 bits générée", ok: true },
      { label: "Empreinte SHA-256 calculée", ok: true },
    ],
  },
  {
    title: "Permissions système",
    description:
      "L'interception du trafic nécessite des règles nftables et l'accès au trust store. Ces élévations de privilèges sont ponctuelles et réversibles.",
    checks: [
      { label: "Accès nftables (root) accordé", ok: true },
      { label: "Installation trust store système possible", ok: true },
    ],
  },
  {
    title: "Test à blanc",
    description:
      "Nous effectuons une activation courte pour vérifier que toute la chaîne fonctionne correctement, puis une désactivation pour confirmer la réversibilité complète.",
    checks: [
      { label: "Activation courte réussie", ok: true },
      { label: "Trafic capturé et attribué", ok: true },
      { label: "Désactivation propre — aucune règle résiduelle", ok: true },
      { label: "Vérification post-désactivation : conforme", ok: true },
    ],
  },
  {
    title: "Prêt à démarrer",
    description:
      "Tous les prérequis sont satisfaits. Vitrail est prêt à surveiller votre trafic réseau. Vous pourrez l'activer à tout moment depuis le tableau de bord.",
    checks: [],
  },
];
