"""Gera um pacote de ícones vetorial (spec 0007 §3) a partir dos glifos de traço.

Uso: python vetor.py esmalte

Lê `fonte/glifos.txt` (os glifos, comuns aos pacotes vetoriais) e
`fonte/<id>.txt` (os metadados do pacote e o estilo), e escreve um SVG por app
em `fonte/<id>-svg/`, `../<id>/pack.json`, `../<id>/apps/*.png` (o SVG
rasterizado) e a folha `fonte/<id>-previa.png`. Só usa a biblioteca padrão: o
rasterizador aqui cobre o que os SVGs usam (caminhos com traço redondo,
preenchimento, gradientes). A saída é sempre a mesma para a mesma fonte.

Formato de `<id>.txt`:

    pack.name: Esmalte              (metadados do pack.json)
    estilo: esmalte                 (esmalte: ladrilho com glifo branco;
                                     traco: só o glifo, numa cor)
    lado: 128                       (o lado do PNG, em pixels)
    cor: #7C818C                    (traco: a cor do glifo)
    cor-escuro: #A9AEB9             (qualquer chave com "-escuro": o valor no
                                     tema escuro; com uma, o pacote ganha
                                     escuro/*.png e o mapa "dark" do pack.json)
    caixa: 0.8125                   (traco: o lado da caixa de 24, na imagem)
    largura: 1.5                    (traco: o traço, na caixa de 24)
"""

import colorsys
import json
import math
import re
import sys
from pathlib import Path

from gerar import codificar

AQUI = Path(__file__).resolve().parent
SS = 4  # amostras por pixel, em cada eixo

# A superelipse dos ladrilhos do dock (`native_dock/paint.rs`, SQUIRCLE), numa
# caixa de 100.
SUPERELIPSE = "M50 0C90 0 100 10 100 50S90 100 50 100 0 90 0 50 10 0 50 0Z"


# --- Fonte -----------------------------------------------------------------


def ler_glifos(caminho):
    glifos, atual = [], None
    for numero, linha in enumerate(caminho.read_text(encoding="utf-8").splitlines(), 1):
        linha = linha.strip()
        if not linha or linha.startswith("#"):
            continue
        if linha.startswith("@"):
            atual = {"id": linha[1:].strip(), "nomes": [], "esmalte": None, "caminhos": [], "onde": numero}
            glifos.append(atual)
        elif linha.startswith("nomes:"):
            atual["nomes"] = [n.strip() for n in linha[6:].split("|") if n.strip()]
        elif linha.startswith("esmalte:"):
            h, s, l = (float(v) for v in linha[8:].split())
            if not (14 <= s <= 48 and 34 <= l <= 46):
                sys.exit(f"{caminho.name}:{numero}: esmalte fora do registro (saturação 14–48, luminosidade 34–46)")
            atual["esmalte"] = (h, s, l)
        else:
            atual["caminhos"].append(linha)
    for glifo in glifos:
        for falta in ("nomes", "esmalte", "caminhos"):
            if not glifo[falta]:
                sys.exit(f"{caminho.name}:{glifo['onde']}: @{glifo['id']} não tem {falta}")
    return glifos


def ler_pacote(caminho):
    meta, estilo = {}, {}
    for linha in caminho.read_text(encoding="utf-8").splitlines():
        linha = linha.strip()
        if not linha or linha.startswith("#"):
            continue
        chave, valor = linha.split(":", 1)
        if chave.startswith("pack."):
            meta[chave[5:].strip()] = valor.strip()
        else:
            estilo[chave.strip()] = valor.strip()
    return meta, estilo


# --- Caminhos --------------------------------------------------------------

FICHA = re.compile(r"[MmLlHhVvCcSsQqAaZz]|-?(?:\d+\.?\d*|\.\d+)")


def achatar(d):
    """Os subcaminhos de `d` como listas de pontos, com se fecham."""
    fichas = FICHA.findall(d)
    i, cmd = 0, None
    x = y = x0 = y0 = 0.0
    ctrl = None
    partes, atual = [], None

    def num():
        nonlocal i
        i += 1
        return float(fichas[i - 1])

    def linha(nx, ny):
        atual[0].append((nx, ny))

    while i < len(fichas):
        if fichas[i].isalpha():
            cmd = fichas[i]
            i += 1
        rel = cmd.islower()
        c = cmd.upper()
        ox, oy = (x, y) if rel else (0.0, 0.0)
        if c == "Z":
            atual[1] = True
            x, y = x0, y0
            ctrl = None
            continue
        if c == "M":
            x, y = ox + num(), oy + num()
            x0, y0 = x, y
            atual = [[(x, y)], False]
            partes.append(atual)
            cmd = "l" if rel else "L"
            ctrl = None
        elif c == "L":
            x, y = ox + num(), oy + num()
            linha(x, y)
            ctrl = None
        elif c == "H":
            x = ox + num()
            linha(x, y)
            ctrl = None
        elif c == "V":
            y = oy + num()
            linha(x, y)
            ctrl = None
        elif c in "CS":
            if c == "C":
                c1 = (ox + num(), oy + num())
            else:
                c1 = (2 * x - ctrl[0], 2 * y - ctrl[1]) if ctrl else (x, y)
            c2 = (ox + num(), oy + num())
            fim = (ox + num(), oy + num())
            for passo in range(1, 25):
                t = passo / 24
                u = 1 - t
                linha(
                    u**3 * x + 3 * u * u * t * c1[0] + 3 * u * t * t * c2[0] + t**3 * fim[0],
                    u**3 * y + 3 * u * u * t * c1[1] + 3 * u * t * t * c2[1] + t**3 * fim[1],
                )
            ctrl = c2
            x, y = fim
        elif c == "Q":
            c1 = (ox + num(), oy + num())
            fim = (ox + num(), oy + num())
            for passo in range(1, 17):
                t = passo / 16
                u = 1 - t
                linha(u * u * x + 2 * u * t * c1[0] + t * t * fim[0], u * u * y + 2 * u * t * c1[1] + t * t * fim[1])
            ctrl = None
            x, y = fim
        elif c == "A":
            rx, ry, giro, grande, horario = num(), num(), num(), num(), num()
            fim = (ox + num(), oy + num())
            for ponto in arco(x, y, rx, ry, giro, grande, horario, *fim):
                linha(*ponto)
            ctrl = None
            x, y = fim
        else:
            sys.exit(f"comando {cmd!r} não suportado em {d!r}")
    return partes


def arco(x1, y1, rx, ry, giro, grande, horario, x2, y2):
    """Os pontos de um arco elíptico do SVG (notação de extremos)."""
    if (x1, y1) == (x2, y2):
        return []
    if rx == 0 or ry == 0:
        return [(x2, y2)]
    rx, ry = abs(rx), abs(ry)
    phi = math.radians(giro)
    cos, sin = math.cos(phi), math.sin(phi)
    dx, dy = (x1 - x2) / 2, (y1 - y2) / 2
    xl, yl = cos * dx + sin * dy, -sin * dx + cos * dy
    escala = xl * xl / (rx * rx) + yl * yl / (ry * ry)
    if escala > 1:
        rx, ry = rx * math.sqrt(escala), ry * math.sqrt(escala)
    num = rx * rx * ry * ry - rx * rx * yl * yl - ry * ry * xl * xl
    den = rx * rx * yl * yl + ry * ry * xl * xl
    k = math.sqrt(max(0.0, num / den))
    if grande == horario:
        k = -k
    cxl, cyl = k * rx * yl / ry, -k * ry * xl / rx
    cx = cos * cxl - sin * cyl + (x1 + x2) / 2
    cy = sin * cxl + cos * cyl + (y1 + y2) / 2
    a1 = math.atan2((yl - cyl) / ry, (xl - cxl) / rx)
    a2 = math.atan2((-yl - cyl) / ry, (-xl - cxl) / rx)
    delta = a2 - a1
    if horario and delta < 0:
        delta += 2 * math.pi
    elif not horario and delta > 0:
        delta -= 2 * math.pi
    passos = max(4, int(abs(delta) / math.radians(4)))
    pontos = []
    for passo in range(1, passos + 1):
        a = a1 + delta * passo / passos
        ex, ey = rx * math.cos(a), ry * math.sin(a)
        pontos.append((cos * ex - sin * ey + cx, sin * ex + cos * ey + cy))
    return pontos


def mover(partes, escala, dx, dy):
    return [[[(px * escala + dx, py * escala + dy) for px, py in pontos], fechado] for pontos, fechado in partes]


# --- Rasterização ----------------------------------------------------------


class Mascara:
    """Uma máscara de amostras (SS × SS por pixel) de uma imagem `lado` × `lado`."""

    def __init__(self, lado):
        self.lado = lado
        self.n = lado * SS
        self.bits = bytearray(self.n * self.n)

    def preencher(self, partes):
        """Preenche os subcaminhos (regra par-ímpar), linha de amostras por linha."""
        arestas = []
        for pontos, _ in partes:
            for a, b in zip(pontos, pontos[1:] + pontos[:1]):
                if a[1] != b[1]:
                    arestas.append((a, b))
        for j in range(self.n):
            sy = (j + 0.5) / SS
            cortes = sorted(
                a[0] + (sy - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
                for a, b in arestas
                if min(a[1], b[1]) <= sy < max(a[1], b[1])
            )
            base = j * self.n
            for x0, x1 in zip(cortes[::2], cortes[1::2]):
                i0 = max(0, math.ceil(x0 * SS - 0.5))
                i1 = min(self.n, math.ceil(x1 * SS - 0.5))
                if i1 > i0:
                    self.bits[base + i0 : base + i1] = b"\x01" * (i1 - i0)

    def tracar(self, partes, largura):
        """Traço de `largura` com pontas e junções redondas: a união das cápsulas."""
        r = largura / 2
        r2 = r * r
        n = self.n
        for pontos, fechado in partes:
            segmentos = list(zip(pontos, pontos[1:]))
            if fechado:
                segmentos.append((pontos[-1], pontos[0]))
            if not segmentos:
                segmentos = [(pontos[0], pontos[0])]
            for (ax, ay), (bx, by) in segmentos:
                vx, vy = bx - ax, by - ay
                vv = vx * vx + vy * vy
                i0 = max(0, int((min(ax, bx) - r) * SS))
                i1 = min(n, int((max(ax, bx) + r) * SS) + 1)
                j0 = max(0, int((min(ay, by) - r) * SS))
                j1 = min(n, int((max(ay, by) + r) * SS) + 1)
                for j in range(j0, j1):
                    sy = (j + 0.5) / SS
                    base = j * n
                    for i in range(i0, i1):
                        sx = (i + 0.5) / SS
                        wx, wy = sx - ax, sy - ay
                        t = 0.0 if vv == 0 else min(1.0, max(0.0, (wx * vx + wy * vy) / vv))
                        dx, dy = wx - t * vx, wy - t * vy
                        if dx * dx + dy * dy <= r2:
                            self.bits[base + i] = 1

    def e(self, outra):
        """As amostras nesta e na `outra`."""
        nova = Mascara(self.lado)
        nova.bits = bytearray(a & b for a, b in zip(self.bits, outra.bits))
        return nova

    def cobertura(self):
        """A fração de cada pixel coberta, de 0 a 1, linha por linha."""
        n, lado, total = self.n, self.lado, SS * SS
        linhas = []
        for y in range(lado):
            soma = [0] * lado
            for j in range(y * SS, y * SS + SS):
                fileira = self.bits[j * n : (j + 1) * n]
                for x in range(lado):
                    soma[x] += sum(fileira[x * SS : x * SS + SS])
            linhas.append([s / total for s in soma])
        return linhas


def hsl(h, s, l):
    r, g, b = colorsys.hls_to_rgb((h % 360) / 360, l / 100, s / 100)
    return (r, g, b)


def sobre(fundo, cor, alfa):
    """`cor` (RGB 0–1) com opacidade `alfa` sobre `fundo` (RGBA 0–1, pré-multiplicado)."""
    r, g, b, a = fundo
    return (cor[0] * alfa + r * (1 - alfa), cor[1] * alfa + g * (1 - alfa), cor[2] * alfa + b * (1 - alfa), alfa + a * (1 - alfa))


def para_png(pixels, lado):
    """Pixels pré-multiplicados (0–1) → linhas RGBA de 8 bits, sem pré-multiplicar."""
    linhas = []
    for y in range(lado):
        linha = bytearray()
        for x in range(lado):
            r, g, b, a = pixels[y][x]
            if a <= 0:
                linha += b"\x00\x00\x00\x00"
                continue
            linha += bytes(min(255, round(c / a * 255)) for c in (r, g, b)) + bytes([min(255, round(a * 255))])
        linhas.append(linha)
    return linhas


# --- Estilos ---------------------------------------------------------------


def geometria_esmalte(lado):
    """Onde vão o ladrilho, a sombra e o glifo numa imagem de `lado`: o ladrilho
    ocupa 7/8 do lado, deixando espaço para a sombra embaixo."""
    t = lado * 7 / 8
    x0 = (lado - t) / 2
    y0 = (lado - t) / 4
    return t, x0, y0


def esmalte(glifo, lado):
    h, s, l = glifo["esmalte"]
    t, x0, y0 = geometria_esmalte(lado)
    k = t / 48  # o ladrilho do dock tem 48 px no tamanho médio
    forma = achatar(SUPERELIPSE)

    ladrilho = Mascara(lado)
    ladrilho.preencher(mover(forma, t / 100, x0, y0))
    sombras = []
    for dy, alfa, cresce in ((1.0, 0.12, 0.0), (3.5, 0.07, 1.5)):
        m = Mascara(lado)
        lado_s = t + 2 * cresce * k
        m.preencher(mover(forma, lado_s / 100, x0 - cresce * k, y0 - cresce * k + dy * k))
        sombras.append((m.cobertura(), alfa))
    borda = Mascara(lado)
    borda.tracar(mover(forma, t / 100, x0, y0), 2 * max(1.0, k * 0.75))
    borda = borda.e(ladrilho)
    traco = Mascara(lado)
    caixa = t * 0.54
    escala = caixa / 24
    for d in glifo["caminhos"]:
        traco.tracar(mover(achatar(d), escala, x0 + (t - caixa) / 2, y0 + (t - caixa) / 2), 1.75 * escala)

    c_ladrilho, c_borda, c_traco = ladrilho.cobertura(), borda.cobertura(), traco.cobertura()
    topo, base = hsl(h, s, l + 7), hsl(h, s, l - 5)
    branco, preto = (1.0, 1.0, 1.0), (0.0, 0.0, 0.0)
    pixels = []
    for y in range(lado):
        linha = []
        for x in range(lado):
            px = (0.0, 0.0, 0.0, 0.0)
            for cobertura, alfa in sombras:
                px = sobre(px, preto, alfa * cobertura[y][x])
            cobre = c_ladrilho[y][x]
            if cobre > 0:
                u = ((x + 0.5 - x0) / t) * 100
                v = ((y + 0.5 - y0) / t) * 100
                f = min(1.0, max(0.0, v / 100))
                cor = tuple(a + (b - a) * f for a, b in zip(topo, base))
                tile = (*cor, 1.0)
                # O brilho: radial-gradient(130% 70% at 50% -12%, branco 0.26, transparente 58%).
                dist = math.hypot((u - 50) / 130, (v + 12) / 70)
                if dist < 0.58:
                    tile = sobre(tile, branco, 0.26 * (1 - dist / 0.58))
                # A borda do esmalte: luz em cima, sombra embaixo.
                if c_borda[y][x] > 0:
                    if v < 5:
                        tile = sobre(tile, branco, 0.32 * (1 - v / 5) * c_borda[y][x] / cobre)
                    elif v > 95:
                        tile = sobre(tile, preto, 0.16 * (v - 95) / 5 * c_borda[y][x] / cobre)
                if c_traco[y][x] > 0:
                    tile = sobre(tile, branco, min(1.0, c_traco[y][x] / cobre))
                px = tuple(a * cobre + b * (1 - cobre) for a, b in zip(tile, px))
            linha.append(px)
        pixels.append(linha)
    return para_png(pixels, lado)


def svg_esmalte(glifo, lado):
    h, s, l = glifo["esmalte"]
    t, x0, y0 = geometria_esmalte(lado)
    k = t / 48
    topo, base = (hex_de(hsl(h, s, l + 7)), hex_de(hsl(h, s, l - 5)))
    caixa = t * 0.54
    escala = caixa / 24
    gx = x0 + (t - caixa) / 2
    gy = y0 + (t - caixa) / 2
    sombras = "".join(
        f'<path d="{SUPERELIPSE}" transform="translate({n(x0 - c * k)} {n(y0 - c * k + dy * k)}) scale({n((t + 2 * c * k) / 100)})" fill="#000" fill-opacity="{a}"/>'
        for dy, a, c in ((1.0, 0.12, 0.0), (3.5, 0.07, 1.5))
    )
    caminhos = "".join(f'<path d="{d}"/>' for d in glifo["caminhos"])
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{lado}" height="{lado}" viewBox="0 0 {lado} {lado}">'
        '<defs>'
        f'<linearGradient id="e" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{topo}"/><stop offset="1" stop-color="{base}"/></linearGradient>'
        '<radialGradient id="b" gradientUnits="userSpaceOnUse" cx="50" cy="-12" r="100" gradientTransform="translate(50 -12) scale(1.3 0.7) translate(-50 12)">'
        '<stop offset="0" stop-color="#fff" stop-opacity=".26"/><stop offset=".58" stop-color="#fff" stop-opacity="0"/></radialGradient>'
        '<linearGradient id="r" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".32"/>'
        '<stop offset=".05" stop-color="#fff" stop-opacity="0"/><stop offset=".95" stop-color="#000" stop-opacity="0"/>'
        '<stop offset="1" stop-color="#000" stop-opacity=".16"/></linearGradient>'
        f'<clipPath id="c"><path d="{SUPERELIPSE}"/></clipPath>'
        '</defs>'
        f"{sombras}"
        f'<g transform="translate({n(x0)} {n(y0)}) scale({n(t / 100)})">'
        f'<path d="{SUPERELIPSE}" fill="url(#e)"/><path d="{SUPERELIPSE}" fill="url(#b)"/>'
        f'<path d="{SUPERELIPSE}" fill="none" stroke="url(#r)" stroke-width="{n(2 * max(1.0, k * 0.75) * 100 / t)}" clip-path="url(#c)"/>'
        "</g>"
        f'<g transform="translate({n(gx)} {n(gy)}) scale({n(escala)})" fill="none" stroke="#fff" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">'
        f"{caminhos}</g></svg>\n"
    )


def geometria_traco(estilo, lado):
    caixa = lado * float(estilo["caixa"])
    return caixa / 24, (lado - caixa) / 2, float(estilo["largura"])


def traco(glifo, lado, estilo):
    escala, margem, largura = geometria_traco(estilo, lado)
    mascara = Mascara(lado)
    for d in glifo["caminhos"]:
        mascara.tracar(mover(achatar(d), escala, margem, margem), largura * escala)
    cor = tuple(int(estilo["cor"][i : i + 2], 16) for i in (1, 3, 5))
    linhas = []
    for fileira in mascara.cobertura():
        linha = bytearray()
        for cobre in fileira:
            linha += bytes(cor) + bytes([round(cobre * 255)]) if cobre > 0 else bytes(4)
        linhas.append(linha)
    return linhas


def svg_traco(glifo, lado, estilo):
    escala, margem, largura = geometria_traco(estilo, lado)
    caminhos = "".join(f'<path d="{d}"/>' for d in glifo["caminhos"])
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{lado}" height="{lado}" viewBox="0 0 {lado} {lado}">'
        f'<g transform="translate({n(margem)} {n(margem)}) scale({n(escala)})" fill="none" stroke="{estilo["cor"]}" '
        f'stroke-width="{largura}" stroke-linecap="round" stroke-linejoin="round">{caminhos}</g></svg>\n'
    )


ESTILOS = {
    "esmalte": (lambda glifo, lado, _: esmalte(glifo, lado), lambda glifo, lado, _: svg_esmalte(glifo, lado)),
    "traco": (traco, svg_traco),
}


def n(v):
    return f"{v:.3f}".rstrip("0").rstrip(".")


def hex_de(cor):
    return "#" + "".join(f"{round(c * 255):02X}" for c in cor)


# --- Prévia ----------------------------------------------------------------


def previa(claras, escuras, lado, colunas=11):
    """As imagens pela metade, lado a lado: as do tema escuro em fundo escuro
    em cima, as do claro em fundo claro embaixo."""
    imagens = claras
    meio = lado // 2
    margem = 8
    celula = meio + margem
    largura = colunas * celula + margem
    metade = -(-len(imagens) // colunas) * celula + margem
    fundos = ((0x28, 0x28, 0x2E), (0xEC, 0xEC, 0xF0))
    tela = [[list(fundos[y >= metade]) for _ in range(largura)] for y in range(metade * 2)]
    for parte, conjunto in ((0, escuras), (1, claras)):
        for indice, linhas in enumerate(conjunto):
            x0 = margem + (indice % colunas) * celula
            y0 = parte * metade + margem + (indice // colunas) * celula
            for y in range(meio):
                for x in range(meio):
                    soma = [0.0, 0.0, 0.0, 0.0]
                    for dy in (0, 1):
                        p = linhas[2 * y + dy]
                        for dx in (0, 1):
                            r, g, b, a = p[(2 * x + dx) * 4 : (2 * x + dx) * 4 + 4]
                            a /= 255
                            soma[0] += r * a
                            soma[1] += g * a
                            soma[2] += b * a
                            soma[3] += a
                    a = soma[3] / 4
                    destino = tela[y0 + y][x0 + x]
                    for c in range(3):
                        destino[c] = round(soma[c] / 4 + destino[c] * (1 - a))
    return codificar(largura, metade * 2, [bytearray(b for px in linha for b in (*px, 255)) for linha in tela])


def main():
    pacote = sys.argv[1] if len(sys.argv) > 1 else "esmalte"
    meta, estilo = ler_pacote(AQUI / f"{pacote}.txt")
    glifos = ler_glifos(AQUI / "glifos.txt")
    lado = int(estilo.get("lado", "128"))
    rasterizar, desenhar = ESTILOS[estilo["estilo"]]
    destino = AQUI.parent / pacote
    # O estilo do tema escuro: cada "chave-escuro" no lugar da "chave".
    claro = {k: v for k, v in estilo.items() if not k.endswith("-escuro")}
    escuro = dict(claro)
    escuro.update({k[: -len("-escuro")]: v for k, v in estilo.items() if k.endswith("-escuro")})
    temas = [("apps", f"{pacote}-svg", claro)]
    if escuro != claro:
        temas.append(("escuro", f"{pacote}-escuro-svg", escuro))
    for pasta, svg in (("apps", f"{pacote}-svg"), ("escuro", f"{pacote}-escuro-svg")):
        for antigo in [*(destino / pasta).glob("*.png"), *(AQUI / svg).glob("*.svg")]:
            antigo.unlink()
    for pasta, svg, _ in temas:
        (destino / pasta).mkdir(parents=True, exist_ok=True)
        (AQUI / svg).mkdir(parents=True, exist_ok=True)

    icones, reserva, escuras = {}, None, {}
    imagens = {pasta: [] for pasta, _, _ in temas}
    for glifo in glifos:
        arquivo = f"apps/{glifo['id']}.png"
        for pasta, svg, deste in temas:
            (AQUI / svg / f"{glifo['id']}.svg").write_text(desenhar(glifo, lado, deste), encoding="utf-8", newline="\n")
            linhas = rasterizar(glifo, lado, deste)
            imagens[pasta].append(linhas)
            (destino / pasta / f"{glifo['id']}.png").write_bytes(codificar(lado, lado, linhas))
            if pasta == "escuro":
                escuras[arquivo] = f"escuro/{glifo['id']}.png"
        for nome in glifo["nomes"]:
            if nome == "reserva":
                reserva = arquivo
            elif nome in icones:
                sys.exit(f"o nome {nome!r} está em @{glifo['id']} e em outro glifo")
            else:
                icones[nome] = arquivo
        print(f"  {glifo['id']}", flush=True)

    manifesto = dict(meta)
    manifesto["icons"] = icones
    if reserva:
        manifesto["fallback"] = reserva
    if escuras:
        manifesto["dark"] = escuras
    texto = json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n"
    (destino / "pack.json").write_text(texto, encoding="utf-8", newline="\n")
    claras = imagens["apps"]
    (AQUI / f"{pacote}-previa.png").write_bytes(previa(claras, imagens.get("escuro", claras), lado))
    print(f"{pacote}: {len(glifos)} glifos, {len(icones)} nomes")


if __name__ == "__main__":
    main()
