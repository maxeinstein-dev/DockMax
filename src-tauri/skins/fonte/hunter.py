"""Gera a skin Caçador (spec 0020 §4): uma barra de couro curtido, com uma
costura pespontada perto das bordas e pontas de osso.

Uso: python hunter.py

Desenhada em vetor (o rasterizador de icon-packs/fonte/vetor.py), em 64 px
de altura, e reduzida pelo dock com suavização (`pixelated: false`). O meio
tem um ponto da costura e se repete, para os pontos não esticarem. Escreve
`../hunter/skin.png` e `../hunter/skin.json`.
"""

import json
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent.parent / "icon-packs" / "fonte"))
from gerar import codificar  # noqa: E402
from vetor import Mascara, achatar, sobre  # noqa: E402

ALTURA = 64
PONTA = 26
MEIO = 8  # um ponto da costura e o vão dele
LARGURA = PONTA + MEIO + PONTA
RAIO = 16

COURO_TOPO = (0x7A, 0x4E, 0x2C)
COURO_BASE = (0x4A, 0x2E, 0x18)
VINCO = (0x2A, 0x18, 0x0C)
LINHA = (0xE8, 0xD2, 0xA6)
OSSO_TOPO = (0xED, 0xE4, 0xCC)
OSSO_BASE = (0xB9, 0xAB, 0x8C)


def cor(rgb):
    return tuple(c / 255 for c in rgb)


def cobertura(desenhar):
    m = Mascara(ALTURA)
    desenhar(m)
    return m.cobertura()


def main():
    w, h, r = LARGURA, ALTURA, RAIO
    corpo = f"M{r} 0H{w - r}a{r} {r} 0 0 1 {r} {r}V{h - r}a{r} {r} 0 0 1 {-r} {r}H{r}a{r} {r} 0 0 1 {-r} {-r}V{r}a{r} {r} 0 0 1 {r} {-r}Z"
    vinco = f"M{r} 1H{w - r}a{r - 1} {r - 1} 0 0 1 {r - 1} {r - 1}V{h - r}a{r - 1} {r - 1} 0 0 1 {-(r - 1)} {r - 1}H{r}a{r - 1} {r - 1} 0 0 1 {-(r - 1)} {-(r - 1)}V{r}a{r - 1} {r - 1} 0 0 1 {r - 1} {-(r - 1)}Z"
    # Os pontos da costura, a 7 px das bordas de cima e de baixo, só entre as
    # pontas de osso: um ponto por 8 px, centrado em cada meio.
    pontos = []
    for y in (7.5, h - 7.5):
        x = PONTA + 2.0
        while x < w - PONTA:
            pontos.append(f"M{x} {y}h4")
            x += MEIO
        # As pontas levam a costura até o osso.
        pontos.append(f"M{PONTA - 8} {y}h4")
        pontos.append(f"M{w - PONTA + 4} {y}h4")
    # As pontas de osso: meia cápsula em cada ponta, com um rebite.
    osso_e = f"M{r} 3H17V{h - 3}H{r}a{r - 3} {r - 3} 0 0 1 {-(r - 3)} {-(r - 3)}V{r}a{r - 3} {r - 3} 0 0 1 {r - 3} {-(r - 3)}Z"
    osso_d = f"M{w - r} 3H{w - 17}V{h - 3}H{w - r}a{r - 3} {r - 3} 0 0 0 {r - 3} {-(r - 3)}V{r}a{r - 3} {r - 3} 0 0 0 {-(r - 3)} {-(r - 3)}Z"
    rebites = [f"M9.5 {h / 2}a1.75 1.75 0 1 1 3.5 0a1.75 1.75 0 1 1 -3.5 0", f"M{w - 13} {h / 2}a1.75 1.75 0 1 1 3.5 0a1.75 1.75 0 1 1 -3.5 0"]

    c_corpo = cobertura(lambda m: m.preencher(achatar(corpo)))
    c_vinco = cobertura(lambda m: m.tracar(achatar(vinco), 1.5))
    c_pontos = cobertura(lambda m: [m.tracar(achatar(d), 1.6) for d in pontos])
    c_osso = cobertura(lambda m: [m.preencher(achatar(d)) for d in (osso_e, osso_d)])
    c_osso_borda = cobertura(lambda m: [m.tracar(achatar(d), 1.2) for d in (osso_e, osso_d)])
    c_rebites = cobertura(lambda m: [m.preencher(achatar(d)) for d in rebites])

    linhas = []
    for y in range(h):
        f = (y + 0.5) / h
        couro = tuple(a + (b - a) * f for a, b in zip(cor(COURO_TOPO), cor(COURO_BASE)))
        osso = tuple(a + (b - a) * f for a, b in zip(cor(OSSO_TOPO), cor(OSSO_BASE)))
        linha = bytearray()
        for x in range(w):
            px = (0.0, 0.0, 0.0, 0.0)
            a = c_corpo[y][x]
            if a > 0:
                px = sobre(px, couro, 0.98 * a)
                px = sobre(px, cor(VINCO), 0.7 * c_vinco[y][x] * a)
                px = sobre(px, cor(LINHA), 0.85 * c_pontos[y][x])
                px = sobre(px, osso, c_osso[y][x])
                px = sobre(px, cor(VINCO), 0.6 * c_osso_borda[y][x])
                px = sobre(px, cor(VINCO), 0.75 * c_rebites[y][x])
            r_, g, b, al = px
            if al <= 0:
                linha += bytes(4)
            else:
                linha += bytes(min(255, round(v / al * 255)) for v in (r_, g, b)) + bytes([min(255, round(al * 255))])
        linhas.append(linha)

    destino = AQUI.parent / "hunter"
    destino.mkdir(exist_ok=True)
    (destino / "skin.png").write_bytes(codificar(w, h, linhas))
    manifesto = {
        "name": "Caçador",
        "image": "skin.png",
        "width": w,
        "height": h,
        "slice": {"top": 0, "right": PONTA, "bottom": 0, "left": PONTA},
        "middle": "repeat",
        "pixelated": False,
    }
    texto = json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n"
    (destino / "skin.json").write_text(texto, encoding="utf-8", newline="\n")
    print(f"hunter: {w}×{h}")


if __name__ == "__main__":
    main()
