# Spikes

Experimentos medidos antes de decisões de arquitetura. Ficam fora do build e da CI do app; cada um é um projeto próprio e o resultado está no ADR que o cita.

- [`native-dock/`](native-dock/): o dock desenhado em Rust nativo (Direct2D), sem WebView2 ([ADR 0011](../docs/adr/0011-spike-dock-nativo.md)). `cargo run --release -- --animate --seconds 30`.
- [`webview-startup/`](webview-startup/): quanto uma janela WebView2 leva para mostrar uma página real do DockMax, a frio e a morno ([spec 0011](../docs/specs/0011-arquitetura-hibrida.md) §2). `cargo run --release -- ../../dist` depois de `pnpm build`.
