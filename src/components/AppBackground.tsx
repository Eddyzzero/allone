import { MeshGradient } from "@paper-design/shaders-react";
import { useTheme } from "../theme";

/**
 * Dégradé animé (Paper Shaders, Apache-2.0) derrière toute l'appli, avec un
 * voile de la couleur de fond pour que les jaquettes et le texte restent lisibles.
 */
export default function AppBackground() {
  const { theme, animate } = useTheme();
  return (
    <div className="app-bg" aria-hidden="true">
      <MeshGradient
        className="app-bg-mesh"
        colors={theme.gradient}
        distortion={0.8}
        swirl={0.15}
        speed={animate ? 0.25 : 0}
        minPixelRatio={1}
        maxPixelCount={1920 * 1080}
      />
      <span className="app-bg-veil" />
    </div>
  );
}
