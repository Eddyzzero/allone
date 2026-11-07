// Icônes du design (tracés repris des maquettes allone).
import type { SVGProps } from "react";

type P = SVGProps<SVGSVGElement> & { size?: number };

function Stroke({ size = 20, children, ...rest }: P) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8}
      strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...rest}>
      {children}
    </svg>
  );
}

export const Play = ({ size = 16, ...p }: P) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...p}><path d="M7 5l12 7-12 7z" /></svg>
);
export const Logo = ({ size = 18 }: P) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M8 5l11 7-11 7z" /></svg>
);
export const Heart = ({ size = 20, filled = true, ...p }: P & { filled?: boolean }) =>
  filled ? (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...p}><path d="M12 20s-7-4.5-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.5-7 10-7 10z" /></svg>
  ) : (
    <Stroke size={size} {...p}><path d="M12 20s-7-4.5-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.5-7 10-7 10z" /></Stroke>
  );
export const Home = (p: P) => <Stroke {...p}><path d="M3 11l9-8 9 8" /><path d="M5 10v10h14V10" /></Stroke>;
export const Books = (p: P) => <Stroke {...p}><rect x="4" y="3" width="6" height="18" rx="1.5" /><rect x="12" y="3" width="4" height="18" rx="1.5" /><path d="M18 4l3 16" /></Stroke>;
export const Pad = (p: P) => <Stroke {...p}><path d="M6 8h12a4 4 0 0 1 4 4v2a3 3 0 0 1-5.4 1.8L15 14H9l-1.6 1.8A3 3 0 0 1 2 14v-2a4 4 0 0 1 4-4z" /><path d="M7 10.5v3M5.5 12h3" /></Stroke>;
export const Sliders = (p: P) => <Stroke {...p}><path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1" /><circle cx="15" cy="6" r="2" /><circle cx="9" cy="12" r="2" /><circle cx="17" cy="18" r="2" /></Stroke>;
export const Search = (p: P) => <Stroke size={18} {...p}><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></Stroke>;
export const Plus = (p: P) => <Stroke {...p}><path d="M12 5v14M5 12h14" /></Stroke>;
export const Close = (p: P) => <Stroke {...p}><path d="M6 6l12 12M18 6L6 18" /></Stroke>;
export const ChevronLeft = (p: P) => <Stroke size={18} strokeWidth={2} {...p}><path d="M15 6l-6 6 6 6" /></Stroke>;
export const ChevronRight = (p: P) => <Stroke size={18} strokeWidth={2} {...p}><path d="M9 6l6 6-6 6" /></Stroke>;
export const Clock = (p: P) => <Stroke size={15} {...p}><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></Stroke>;
export const Folder = (p: P) => <Stroke size={18} {...p}><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></Stroke>;
export const File = (p: P) => <Stroke size={18} {...p}><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" /><path d="M14 3v5h5" /></Stroke>;
export const Dice = (p: P) => <Stroke size={18} {...p}><rect x="4" y="4" width="16" height="16" rx="3" /><circle cx="9" cy="9" r="1.2" fill="currentColor" /><circle cx="15" cy="15" r="1.2" fill="currentColor" /><circle cx="15" cy="9" r="1.2" fill="currentColor" /><circle cx="9" cy="15" r="1.2" fill="currentColor" /></Stroke>;
export const Sort = (p: P) => <Stroke size={16} {...p}><path d="M7 4v16M4 17l3 3 3-3M14 6h7M14 12h5M14 18h3" /></Stroke>;
export const Grid = (p: P) => <Stroke size={16} {...p}><rect x="4" y="4" width="6" height="6" rx="1" /><rect x="14" y="4" width="6" height="6" rx="1" /><rect x="4" y="14" width="6" height="6" rx="1" /><rect x="14" y="14" width="6" height="6" rx="1" /></Stroke>;
export const List = (p: P) => <Stroke size={16} {...p}><path d="M9 6h11M9 12h11M9 18h11" /><circle cx="5" cy="6" r="1" fill="currentColor" /><circle cx="5" cy="12" r="1" fill="currentColor" /><circle cx="5" cy="18" r="1" fill="currentColor" /></Stroke>;
export const Check = (p: P) => <Stroke strokeWidth={2.4} {...p}><path d="M5 12.5l4.5 4.5L19 7.5" /></Stroke>;
export const Upload = (p: P) => <Stroke size={24} {...p}><path d="M12 16V4M7 9l5-5 5 5" /><path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3" /></Stroke>;
export const Terminal = (p: P) => <Stroke size={18} {...p}><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M7 9l3 3-3 3M13 15h4" /></Stroke>;
export const User = (p: P) => <Stroke size={18} {...p}><circle cx="12" cy="8" r="4" /><path d="M4 20c0-4 3.6-6 8-6s8 2 8 6" /></Stroke>;
