import { createContext, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useStore } from "../store";
import { padFamily, padName, shouldFocusAfterNav, type PadFamily } from "./logic";
import { activate, focusFirstIn, focusRoot, goBack, moveFocus, switchSection } from "./navigation";
import { createDriverSelector, createPadReader, type PadAction } from "./reader";

interface PadInfo {
  name: string;
  family: PadFamily;
}
interface GamepadState {
  pad: PadInfo | null;
  active: boolean;
}

const GamepadContext = createContext<GamepadState>({ pad: null, active: false });

/** Manette connectée (nom, famille) et indicateur « la manette a servi depuis la dernière souris/clavier ». */
export function useGamepad(): GamepadState {
  return useContext(GamepadContext);
}

const hasApi = () => typeof navigator !== "undefined" && typeof navigator.getGamepads === "function";

function connectedPads(): Gamepad[] {
  if (!hasApi()) return [];
  return Array.from(navigator.getGamepads()).filter((g): g is Gamepad => !!g && g.connected);
}

const toInfo = (g: Gamepad): PadInfo => ({ name: padName(g.id), family: padFamily(g.id) });

export function GamepadProvider({ children }: { children: ReactNode }) {
  const { running, notify } = useStore();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const [pad, setPad] = useState<PadInfo | null>(null);
  const [active, setActive] = useState(false);
  const [connected, setConnected] = useState(false);

  // Valeurs courantes lues par la boucle sans la relancer.
  const runningRef = useRef(running);
  runningRef.current = running;
  const pathRef = useRef(pathname);
  pathRef.current = pathname;
  const navRef = useRef(navigate);
  navRef.current = navigate;
  const notifyRef = useRef(notify);
  notifyRef.current = notify;
  // Chemin au moment où une action manette a pu changer de page (null = rien d'armé).
  const focusAfterNav = useRef<string | null>(null);
  const armTimer = useRef(0);
  const armFocusAfterNav = () => {
    focusAfterNav.current = pathRef.current;
    window.clearTimeout(armTimer.current);
    // Si la page ne change pas, on désarme pour ne pas voler le focus plus tard.
    armTimer.current = window.setTimeout(() => {
      focusAfterNav.current = null;
    }, 400);
  };
  const readerRef = useRef(createPadReader());

  // Après un changement de page à la manette : focus sur le contenu.
  useEffect(() => {
    if (shouldFocusAfterNav(focusAfterNav.current, pathname)) focusFirstIn("main");
    focusAfterNav.current = null;
    window.clearTimeout(armTimer.current);
  }, [pathname]);

  // Connexion / déconnexion.
  useEffect(() => {
    if (!hasApi()) return;
    const refresh = () => setConnected(connectedPads().length > 0);
    const onConnect = (e: GamepadEvent) => {
      setPad(toInfo(e.gamepad));
      notifyRef.current(`Manette connectée : ${padName(e.gamepad.id)}`, "info");
      refresh();
    };
    const onDisconnect = () => {
      readerRef.current.reset();
      const rest = connectedPads();
      setPad(rest.length > 0 ? toInfo(rest[rest.length - 1]) : null);
      if (rest.length === 0) setActive(false);
      notifyRef.current("Manette déconnectée", "info");
      refresh();
    };
    window.addEventListener("gamepadconnected", onConnect);
    window.addEventListener("gamepaddisconnected", onDisconnect);
    const already = connectedPads();
    if (already.length > 0) {
      setPad(toInfo(already[already.length - 1]));
      setConnected(true);
    }
    return () => {
      window.removeEventListener("gamepadconnected", onConnect);
      window.removeEventListener("gamepaddisconnected", onDisconnect);
    };
  }, []);

  // Souris / clavier : on quitte le mode manette.
  useEffect(() => {
    const off = (e: Event) => {
      // Les évènements synthétiques (Échap / flèches émis par la manette) ne comptent pas.
      if (!e.isTrusted) return;
      if (document.documentElement.dataset.input === "pad") delete document.documentElement.dataset.input;
      setActive(false);
    };
    const events = ["mousemove", "keydown", "pointerdown"] as const;
    events.forEach((n) => window.addEventListener(n, off, true));
    return () => events.forEach((n) => window.removeEventListener(n, off, true));
  }, []);

  // Boucle de lecture, seulement quand une manette est connectée.
  useEffect(() => {
    if (!connected || !hasApi()) return;
    const reader = readerRef.current;
    const selector = createDriverSelector();
    let raf = 0;

    const run = (a: PadAction) => {
      document.documentElement.dataset.input = "pad";
      setActive(true);
      switch (a.type) {
        case "move":
          moveFocus(a.dir, { repeat: a.repeat });
          break;
        case "confirm":
          armFocusAfterNav();
          activate();
          break;
        case "back":
          if (focusRoot() === document.body) armFocusAfterNav();
          goBack((d) => navRef.current(d), pathRef.current);
          break;
        case "section":
          if (switchSection(a.delta)) armFocusAfterNav();
          break;
      }
    };

    const tick = (now: number) => {
      try {
      const pads = connectedPads();
      const snaps = pads.map((g) => ({
        index: g.index,
        snap: { buttons: g.buttons.map((b) => b.pressed), axes: Array.from(g.axes) },
      }));
      const { driver, switched } = selector.pick(snaps);
      const cur = snaps.find((p) => p.index === driver);
      if (cur) {
        if (switched) {
          // Nouvelle manette pilote : ce qu'elle tient déjà ne déclenche rien.
          reader.swallowHeld(cur.snap);
          const g = pads.find((p) => p.index === driver);
          if (g) setPad(toInfo(g));
        } else {
          const ignored = runningRef.current.size > 0 || !document.hasFocus();
          reader.read(cur.snap, now, ignored).forEach(run);
        }
      }
      } finally {
        // Une exception dans une action ne doit pas arrêter la manette.
        raf = requestAnimationFrame(tick);
      }
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [connected]);

  const value = useMemo(() => ({ pad, active }), [pad, active]);
  return <GamepadContext.Provider value={value}>{children}</GamepadContext.Provider>;
}
