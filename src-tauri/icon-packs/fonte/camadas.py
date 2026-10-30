"""Os estilos de vetor.py feitos de camadas: cada estilo diz, para um glifo, uma
lista de camadas, e daqui saem o PNG e o SVG da mesma lista.

Uma camada pinta uma forma (a placa em superelipse, o disco ou o glifo) com
uma tinta (cor lisa, gradiente vertical ou o brilho do esmalte), deslocada,
menos outra forma deslocada (para as bordas de luz e de sombra), desfocada e
recortada pela placa. Só a biblioteca padrão.
"""

import math

from vetor import SUPERELIPSE, Mascara, achatar, hex_de, hsl, mover, n, sobre, geometria_esmalte

# --- Formas ----------------------------------------------------------------


class Forma:
    """Uma forma numa imagem de `lado`: `placa`, `disco` ou `glifo`."""

    def __init__(self, tipo, lado, glifo=None, caixa=0.54, largura=1.75, nucleo=1.0, recuo=0.0):
        self.tipo, self.lado = tipo, lado
        self.glifo, self.caixa, self.largura, self.nucleo = glifo, caixa, largura, nucleo
        # `octogono`: quanto a forma entra, em pixels, de cada lado.
        self.recuo = recuo
        self.t, self.x0, self.y0 = geometria_esmalte(lado)

    def chave(self):
        return (self.tipo, self.caixa, self.largura, self.nucleo, self.recuo)

    def octogono(self, dx=0.0, dy=0.0):
        """O caminho do octógono: o quadrado da placa com os cantos
        chanfrados a 45°, em 29% do lado."""
        s = self.t - 2 * self.recuo
        x, y = self.x0 + self.recuo + dx, self.y0 + self.recuo + dy
        c = s * 0.29
        return (
            f"M{n(x + c)} {n(y)}H{n(x + s - c)}L{n(x + s)} {n(y + c)}V{n(y + s - c)}"
            f"L{n(x + s - c)} {n(y + s)}H{n(x + c)}L{n(x)} {n(y + s - c)}V{n(y + c)}Z"
        )

    def caixa_glifo(self):
        """O lado da caixa de 24 do glifo e onde ela começa."""
        lado = self.t * self.caixa
        return lado, self.x0 + (self.t - lado) / 2, self.y0 + (self.t - lado) / 2

    def mascara(self, dx=0.0, dy=0.0):
        m = Mascara(self.lado)
        if self.tipo == "placa":
            m.preencher(mover(achatar(SUPERELIPSE), self.t / 100, self.x0 + dx, self.y0 + dy))
        elif self.tipo == "octogono":
            m.preencher(achatar(self.octogono(dx, dy)))
        elif self.tipo == "disco":
            r = self.t / 2 - self.recuo
            cx, cy = self.x0 + self.t / 2 + dx, self.y0 + self.t / 2 + dy
            m.preencher(achatar(f"M{cx} {cy - r}a{r} {r} 0 1 1 0 {2 * r}a{r} {r} 0 1 1 0 {-2 * r}z"))
        else:
            lado, gx, gy = self.caixa_glifo()
            escala = lado / 24
            for d in self.glifo["caminhos"]:
                m.tracar(mover(achatar(d), escala, gx + dx, gy + dy), self.largura * self.nucleo * escala)
        return m.cobertura()

    def svg(self, dx=0.0, dy=0.0, tinta=""):
        """A forma em SVG; `tinta` são os atributos de cor."""
        if self.tipo == "placa":
            return f'<path d="{SUPERELIPSE}" transform="translate({n(self.x0 + dx)} {n(self.y0 + dy)}) scale({n(self.t / 100)})" {tinta}/>'
        if self.tipo == "octogono":
            return f'<path d="{self.octogono(dx, dy)}" {tinta}/>'
        if self.tipo == "disco":
            r = self.t / 2 - self.recuo
            return f'<circle cx="{n(self.x0 + self.t / 2 + dx)}" cy="{n(self.y0 + self.t / 2 + dy)}" r="{n(r)}" {tinta}/>'
        lado, gx, gy = self.caixa_glifo()
        escala = lado / 24
        caminhos = "".join(f'<path d="{d}"/>' for d in self.glifo["caminhos"])
        return (
            f'<g transform="translate({n(gx + dx)} {n(gy + dy)}) scale({n(escala)})" fill="none" {tinta} '
            f'stroke-width="{n(self.largura * self.nucleo)}" stroke-linecap="round" stroke-linejoin="round">{caminhos}</g>'
        )


class Camada:
    """`forma` com `tinta` e opacidade `alfa`, deslocada (`dx`, `dy`), menos a
    forma `menos` deslocada, desfocada `borrao` px e recortada por `recorte`."""

    def __init__(self, forma, tinta, alfa=1.0, dx=0.0, dy=0.0, menos=None, borrao=0, recorte=None):
        self.forma, self.tinta, self.alfa = forma, tinta, alfa
        self.dx, self.dy, self.menos, self.borrao, self.recorte = dx, dy, menos, borrao, recorte


# Tintas: ("cor", rgb), ("vertical", rgb do topo, rgb da base) na altura da
# placa, ("brilho",) o brilho radial branco do esmalte.


def borrar(cobertura, raio):
    """Três passadas de média móvel em cada eixo: quase um desfoque gaussiano."""
    if raio <= 0:
        return cobertura
    lado = len(cobertura)
    atual = [list(linha) for linha in cobertura]
    for _ in range(3):
        for eixo in (0, 1):
            novo = [[0.0] * lado for _ in range(lado)]
            for i in range(lado):
                linha = atual[i] if eixo == 0 else [atual[j][i] for j in range(lado)]
                soma = sum(linha[0 : raio + 1]) + linha[0] * raio
                saida = []
                for k in range(lado):
                    saida.append(soma / (2 * raio + 1))
                    entra = linha[min(lado - 1, k + raio + 1)]
                    sai = linha[max(0, k - raio)]
                    soma += entra - sai
                for k in range(lado):
                    if eixo == 0:
                        novo[i][k] = saida[k]
                    else:
                        novo[k][i] = saida[k]
            atual = novo
    return atual


def rasterizar(camadas, lado):
    """As camadas, de baixo para cima, em linhas RGBA de 8 bits."""
    pixels = [[(0.0, 0.0, 0.0, 0.0)] * lado for _ in range(lado)]
    cache = {}

    def mascara(forma, dx, dy):
        chave = (forma.chave(), dx, dy)
        if chave not in cache:
            cache[chave] = forma.mascara(dx, dy)
        return cache[chave]

    for c in camadas:
        cob = mascara(c.forma, c.dx, c.dy)
        if c.menos is not None:
            forma, mdx, mdy = c.menos
            outra = mascara(forma, mdx, mdy)
            cob = [[a * (1 - b) for a, b in zip(la, lb)] for la, lb in zip(cob, outra)]
        cob = borrar(cob, c.borrao)
        if c.recorte is not None:
            corte = mascara(c.recorte, 0.0, 0.0)
            cob = [[a * b for a, b in zip(la, lb)] for la, lb in zip(cob, corte)]
        f = c.forma
        for y in range(lado):
            linha, fila = pixels[y], cob[y]
            for x in range(lado):
                a = fila[x]
                if a <= 0:
                    continue
                cor, alfa = tinta_em(c.tinta, f, x, y)
                linha[x] = sobre(linha[x], cor, min(1.0, a) * alfa * c.alfa)
    linhas = []
    for linha in pixels:
        saida = bytearray()
        for r, g, b, a in linha:
            if a <= 0:
                saida += bytes(4)
            else:
                saida += bytes(min(255, round(v / a * 255)) for v in (r, g, b)) + bytes([min(255, round(a * 255))])
        linhas.append(saida)
    return linhas


def tinta_em(tinta, forma, x, y):
    if tinta[0] == "cor":
        return tinta[1], 1.0
    v = ((y + 0.5 - forma.y0) / forma.t) * 100
    if tinta[0] == "vertical":
        f = min(1.0, max(0.0, v / 100))
        return tuple(a + (b - a) * f for a, b in zip(tinta[1], tinta[2])), 1.0
    u = ((x + 0.5 - forma.x0) / forma.t) * 100
    dist = math.hypot((u - 50) / 130, (v + 12) / 70)
    return (1.0, 1.0, 1.0), (0.26 * (1 - dist / 0.58) if dist < 0.58 else 0.0)


def svg(camadas, lado):
    """As mesmas camadas em SVG."""
    defs, corpo = [], []
    for i, c in enumerate(camadas):
        f = c.forma
        if c.tinta[0] == "cor":
            pinta = hex_de(c.tinta[1])
        else:
            gid = f"t{i}"
            if c.tinta[0] == "vertical":
                defs.append(
                    f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="0" y1="{n(f.y0)}" x2="0" y2="{n(f.y0 + f.t)}">'
                    f'<stop offset="0" stop-color="{hex_de(c.tinta[1])}"/><stop offset="1" stop-color="{hex_de(c.tinta[2])}"/></linearGradient>'
                )
            else:
                k = f.t / 100
                defs.append(
                    f'<radialGradient id="{gid}" gradientUnits="userSpaceOnUse" cx="{n(f.x0 + 50 * k)}" cy="{n(f.y0 - 12 * k)}" r="{n(100 * k)}" '
                    f'gradientTransform="translate({n(f.x0 + 50 * k)} {n(f.y0 - 12 * k)}) scale(1.3 0.7) translate({n(-(f.x0 + 50 * k))} {n(-(f.y0 - 12 * k))})">'
                    '<stop offset="0" stop-color="#fff" stop-opacity=".26"/><stop offset=".58" stop-color="#fff" stop-opacity="0"/></radialGradient>'
                )
            pinta = f"url(#{gid})"
        atributo = f'stroke="{pinta}"' if f.tipo == "glifo" else f'fill="{pinta}"'
        elemento = f.svg(c.dx, c.dy, atributo)
        if c.menos is not None:
            forma, mdx, mdy = c.menos
            preto = 'stroke="#000"' if forma.tipo == "glifo" else 'fill="#000"'
            defs.append(
                f'<mask id="m{i}" maskUnits="userSpaceOnUse" x="0" y="0" width="{lado}" height="{lado}">'
                f'<rect width="{lado}" height="{lado}" fill="#fff"/>{forma.svg(mdx, mdy, preto)}</mask>'
            )
            elemento = f'<g mask="url(#m{i})">{elemento}</g>'
        if c.borrao:
            defs.append(f'<filter id="b{i}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="{n(c.borrao * 1.0)}"/></filter>')
            elemento = f'<g filter="url(#b{i})">{elemento}</g>'
        if c.recorte is not None:
            defs.append(f'<clipPath id="c{i}">{c.recorte.svg()}</clipPath>')
            elemento = f'<g clip-path="url(#c{i})">{elemento}</g>'
        corpo.append(f'<g opacity="{n(c.alfa)}">{elemento}</g>' if c.alfa < 1 else elemento)
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{lado}" height="{lado}" viewBox="0 0 {lado} {lado}">'
        f'<defs>{"".join(defs)}</defs>{"".join(corpo)}</svg>\n'
    )


# --- Estilos ---------------------------------------------------------------

BRANCO, PRETO = (1.0, 1.0, 1.0), (0.0, 0.0, 0.0)


def numero(estilo, chave):
    return float(estilo[chave])


def circulo(glifo, lado, estilo):
    """Disco chapado na cor do app, glifo branco e uma sombra curta."""
    h, s, l = glifo["esmalte"]
    disco = Forma("disco", lado)
    traco = Forma("glifo", lado, glifo, caixa=numero(estilo, "caixa"))
    k = disco.t / 48
    fundo = hsl(h, min(60.0, s * numero(estilo, "saturacao")), l + numero(estilo, "luz"))
    return [
        Camada(disco, ("cor", PRETO), 0.22, dy=1.5 * k, borrao=2),
        Camada(disco, ("cor", fundo)),
        Camada(traco, ("cor", BRANCO)),
    ]


def tom(glifo, lado, estilo):
    """Placa clara (ou escura) da matiz do app e glifo da mesma matiz."""
    h, s, _ = glifo["esmalte"]
    placa = Forma("placa", lado)
    traco = Forma("glifo", lado, glifo)
    sat = min(60.0, s * numero(estilo, "saturacao"))
    return [
        Camada(placa, ("cor", hsl(h, sat, numero(estilo, "fundo")))),
        Camada(traco, ("cor", hsl(h, min(70.0, sat * 1.4), numero(estilo, "glifo")))),
    ]


def vidro(glifo, lado, estilo):
    """Placa translúcida tingida da matiz, borda de luz, brilho e glifo branco."""
    h, s, _ = glifo["esmalte"]
    placa = Forma("placa", lado)
    traco = Forma("glifo", lado, glifo)
    k = placa.t / 48
    cor = hsl(h, min(60.0, s * 1.3), 52)
    return [
        Camada(placa, ("cor", PRETO), 0.10, dy=1.5 * k, borrao=3),
        Camada(placa, ("cor", cor), numero(estilo, "opacidade")),
        Camada(placa, ("vertical", BRANCO, cor), 0.18),
        Camada(placa, ("brilho",)),
        Camada(placa, ("cor", BRANCO), 0.5, menos=(placa, 0.0, 1.2 * k), recorte=placa),
        Camada(traco, ("cor", PRETO), 0.22, dy=0.5 * k, borrao=1),
        Camada(traco, ("cor", BRANCO)),
    ]


def neon(glifo, lado, estilo):
    """Glifo em tubo de neon da cor do app, com o brilho em volta; no tema
    claro, sobre uma placa escura."""
    h, _, _ = glifo["esmalte"]
    placa = Forma("placa", lado)
    traco = Forma("glifo", lado, glifo, caixa=numero(estilo, "caixa"))
    nucleo = Forma("glifo", lado, glifo, caixa=numero(estilo, "caixa"), nucleo=0.45)
    cor, luz = hsl(h, 95, 58), hsl(h, 100, 86)
    camadas = []
    if estilo.get("placa") == "sim":
        camadas += [
            Camada(placa, ("cor", PRETO), 0.18, dy=2, borrao=2),
            Camada(placa, ("vertical", (0.10, 0.105, 0.13), (0.06, 0.063, 0.08))),
            Camada(placa, ("cor", BRANCO), 0.10, menos=(placa, 0.0, 2.0), recorte=placa),
        ]
    return camadas + [
        Camada(traco, ("cor", cor), 0.9, borrao=5),
        Camada(traco, ("cor", cor), 0.6, borrao=2),
        Camada(traco, ("cor", cor)),
        Camada(nucleo, ("cor", luz)),
    ]


def argila(glifo, lado, estilo):
    """Placa fofa de argila na cor do app: luz por dentro em cima e à
    esquerda, sombra por dentro embaixo e à direita, sombra macia no chão e
    glifo branco grosso em relevo."""
    h, s, l = glifo["esmalte"]
    placa = Forma("placa", lado)
    traco = Forma("glifo", lado, glifo, largura=2.4)
    k = placa.t / 48
    fundo = hsl(h, min(62.0, s * 1.2), l + 14)
    return [
        Camada(placa, ("cor", PRETO), 0.26, dy=2.5 * k, borrao=5),
        Camada(placa, ("cor", fundo)),
        Camada(placa, ("cor", BRANCO), 0.55, menos=(placa, 1.6 * k, 2.0 * k), borrao=3, recorte=placa),
        Camada(placa, ("cor", PRETO), 0.20, menos=(placa, -1.6 * k, -2.0 * k), borrao=3, recorte=placa),
        Camada(traco, ("cor", PRETO), 0.22, dx=0.6 * k, dy=0.9 * k, borrao=2),
        Camada(traco, ("cor", BRANCO)),
    ]


def cor_hex(texto):
    return tuple(int(texto[i : i + 2], 16) / 255 for i in (1, 3, 5))


def tatico(glifo, lado, estilo):
    """Placa octogonal verde-oliva escura, com fio claro na borda, e o glifo
    de contorno em areia, como um ícone de HUD (spec 0020 §4)."""
    placa = Forma("octogono", lado)
    dentro = Forma("octogono", lado, recuo=lado / 64)
    traco = Forma("glifo", lado, glifo, caixa=numero(estilo, "caixa"))
    k = placa.t / 48
    areia = cor_hex(estilo["cor"])
    return [
        Camada(placa, ("cor", PRETO), 0.28, dy=1.5 * k, borrao=2),
        Camada(placa, ("vertical", cor_hex(estilo["placa"]), cor_hex(estilo["placa-base"]))),
        Camada(placa, ("cor", areia), 0.4, menos=(dentro, 0.0, 0.0)),
        Camada(traco, ("cor", areia)),
    ]


def cacador(glifo, lado, estilo):
    """Medalhão de couro com aro de osso e o glifo gravado em âmbar, em
    relevo: sombra embaixo e à direita, luz em cima e à esquerda (spec 0020
    §4)."""
    k = geometria_esmalte(lado)[0] / 48
    aro = Forma("disco", lado)
    couro = Forma("disco", lado, recuo=4.5 * k)
    vinco = Forma("disco", lado, recuo=5.5 * k)
    traco = Forma("glifo", lado, glifo, caixa=numero(estilo, "caixa"), largura=2.2)
    ambar = cor_hex(estilo["cor"])
    return [
        Camada(aro, ("cor", PRETO), 0.3, dy=1.5 * k, borrao=3),
        Camada(aro, ("vertical", cor_hex(estilo["osso"]), cor_hex(estilo["osso-base"]))),
        Camada(couro, ("vertical", cor_hex(estilo["couro"]), cor_hex(estilo["couro-base"]))),
        Camada(couro, ("cor", PRETO), 0.45, menos=(vinco, 0.0, 0.0)),
        Camada(traco, ("cor", PRETO), 0.55, dx=0.6 * k, dy=0.8 * k, borrao=1),
        Camada(traco, ("cor", BRANCO), 0.25, dx=-0.4 * k, dy=-0.5 * k),
        Camada(traco, ("cor", ambar)),
    ]


ESTILOS = {"cacador": cacador, "tatico": tatico, "circulo": circulo, "tom": tom, "vidro": vidro, "neon": neon, "argila": argila}
