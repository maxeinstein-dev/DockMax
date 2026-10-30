"""Gera a skin Tático (spec 0020 §4): uma barra carvão com toque de
verde-oliva, cantos chanfrados a 45°, um fio cor de areia no topo, como o
de um HUD, e colchetes de mira nas pontas.

Uso: python tactical.py

Desenhada em vetor (o rasterizador de icon-packs/fonte/vetor.py), em 64 px
de altura, e reduzida pelo dock com suavização (`pixelated: false`). Escreve
`../tactical/skin.png` e `../tactical/skin.json`.
"""

import json
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent.parent / "icon-packs" / "fonte"))
from gerar import codificar  # noqa: E402
from vetor import Mascara, achatar, sobre  # noqa: E402

ALTURA = 64
PONTA = 24  # cada ponta, em pixels
MEIO = 4
LARGURA = PONTA + MEIO + PONTA
CHANFRO = 10

CARVAO_TOPO = (0x2D, 0x31, 0x25)
CARVAO_BASE = (0x19, 0x1B, 0x13)
BORDA = (0x4A, 0x50, 0x38)
AREIA = (0xD9, 0xC9, 0xA3)


def cor(rgb):
    return tuple(c / 255 for c in rgb)


def cobertura(desenhar):
    m = Mascara(ALTURA)
    desenhar(m)
    return m.cobertura()


def main():
    w, h, c = LARGURA, ALTURA, CHANFRO
    corpo = (
        f"M{c} 0H{w - c}L{w} {c}V{h - c}L{w - c} {h}H{c}L0 {h - c}V{c}Z"
    )
    borda = (
        f"M{c + 0.5} 0.75H{w - c - 0.5}L{w - 0.75} {c + 0.5}V{h - c - 0.5}"
        f"L{w - c - 0.5} {h - 0.75}H{c + 0.5}L0.75 {h - c - 0.5}V{c + 0.5}Z"
    )
    fio = f"M{c + 4} 5.5H{w - c - 4}"
    colchetes = [f"M15 18H9V46H15", f"M{w - 15} 18H{w - 9}V46H{w - 15}"]

    c_corpo = cobertura(lambda m: m.preencher(achatar(corpo)))
    c_borda = cobertura(lambda m: m.tracar(achatar(borda), 1.5))
    c_fio = cobertura(lambda m: m.tracar(achatar(fio), 1.5))
    c_colchetes = cobertura(lambda m: [m.tracar(achatar(d), 2.0) for d in colchetes])

    linhas = []
    for y in range(h):
        f = (y + 0.5) / h
        fundo = tuple(a + (b - a) * f for a, b in zip(cor(CARVAO_TOPO), cor(CARVAO_BASE)))
        linha = bytearray()
        for x in range(w):
            px = (0.0, 0.0, 0.0, 0.0)
            a = c_corpo[y][x]
            if a > 0:
                px = sobre(px, fundo, 0.97 * a)
                px = sobre(px, cor(BORDA), c_borda[y][x] * a)
                px = sobre(px, cor(AREIA), 0.75 * c_fio[y][x])
                px = sobre(px, cor(AREIA), 0.85 * c_colchetes[y][x])
            r, g, b, al = px
            if al <= 0:
                linha += bytes(4)
            else:
                linha += bytes(min(255, round(v / al * 255)) for v in (r, g, b)) + bytes([min(255, round(al * 255))])
        linhas.append(linha)

    destino = AQUI.parent / "tactical"
    destino.mkdir(exist_ok=True)
    (destino / "skin.png").write_bytes(codificar(w, h, linhas))
    manifesto = {
        "name": "Tático",
        "image": "skin.png",
        "width": w,
        "height": h,
        "slice": {"top": 0, "right": PONTA, "bottom": 0, "left": PONTA},
        "middle": "stretch",
        "pixelated": False,
    }
    texto = json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n"
    (destino / "skin.json").write_text(texto, encoding="utf-8", newline="\n")
    print(f"tactical: {w}×{h}")


if __name__ == "__main__":
    main()
