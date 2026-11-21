import { Link } from "react-router-dom";
import { Bag, Pad } from "./Icons";

/** Bascule entre la bibliothèque (« Mes jeux ») et la boutique Steam. */
export default function InterfaceSwitch({ current }: { current: "library" | "store" }) {
  return (
    <nav className="mode-switch" aria-label="Interface">
      <Link to="/" aria-current={current === "library" ? "page" : undefined}>
        <Pad size={18} />Mes jeux
      </Link>
      <Link to="/store" aria-current={current === "store" ? "page" : undefined}>
        <Bag />Boutique Steam
      </Link>
    </nav>
  );
}
