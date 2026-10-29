"""Gera um pacote de ícones (spec 0007 §3) a partir dos desenhos em texto.

Uso: python gerar.py retro-8bit

Lê `fonte/<id>.txt` e escreve `../<id>/pack.json`, `../<id>/apps/*.png` e uma
folha de prévia, `fonte/<id>-previa.png` (os desenhos em fundo escuro e claro). Só
usa a biblioteca padrão. A saída é sempre a mesma para a mesma fonte; um
teste do Rust (`persistence::bundled_packs`) confere que os PNGs do repositório
são os desenhos da fonte.

Formato da fonte:

    pack.name: Retrô 8-bit          (metadados do pack.json)
    pack.scale: 8                   (cada pixel vira 8×8; 16 → 128 px)
    cor.0: #000000                  (a paleta: um caractere por cor)

    @chrome                         (o nome do arquivo, apps/chrome.png)
    nomes: Google Chrome | chrome   (as chaves do pack.json; "reserva" = fallback)
    .....000000.....                (16 linhas de 16 caracteres; "." é transparente)
"""

import json
import struct
import sys
import zlib
from pathlib import Path

AQUI = Path(__file__).resolve().parent
LADO = 16


def ler(caminho):
    meta, paleta, desenhos = {}, {}, []
    atual = None
    for numero, linha in enumerate(caminho.read_text(encoding="utf-8").splitlines(), 1):
        linha = linha.rstrip()
        if not linha or linha.startswith("#"):
            continue
        if linha.startswith("pack."):
            chave, valor = linha[5:].split(":", 1)
            meta[chave.strip()] = valor.strip()
        elif linha.startswith("cor."):
            chave, valor = linha[4:].split(":", 1)
            paleta[chave.strip()] = valor.strip()
        elif linha.startswith("@"):
            atual = {"id": linha[1:].strip(), "nomes": [], "linhas": [], "onde": numero}
            desenhos.append(atual)
        elif linha.startswith("nomes:"):
            atual["nomes"] = [n.strip() for n in linha[6:].split("|") if n.strip()]
        else:
            if len(linha) != LADO:
                sys.exit(f"{caminho.name}:{numero}: a linha tem {len(linha)} caracteres, não {LADO}")
            desconhecida = set(linha) - set(paleta) - {"."}
            if desconhecida:
                sys.exit(f"{caminho.name}:{numero}: cor fora da paleta: {''.join(sorted(desconhecida))}")
            atual["linhas"].append(linha)
    for desenho in desenhos:
        if len(desenho["linhas"]) != LADO:
            sys.exit(f"{caminho.name}:{desenho['onde']}: @{desenho['id']} tem {len(desenho['linhas'])} linhas, não {LADO}")
        if not desenho["nomes"]:
            sys.exit(f"{caminho.name}:{desenho['onde']}: @{desenho['id']} não tem nomes")
    return meta, paleta, desenhos


def rgba(cor):
    return bytes(int(cor[i : i + 2], 16) for i in (1, 3, 5)) + b"\xff"


def codificar(largura, altura, linhas_rgba):
    """Um PNG RGBA de 8 bits a partir das linhas de pixels, sem filtro."""

    def bloco(tipo, dados):
        return struct.pack(">I", len(dados)) + tipo + dados + struct.pack(">I", zlib.crc32(tipo + dados))

    cru = b"".join(b"\x00" + bytes(linha) for linha in linhas_rgba)
    return (
        b"\x89PNG\r\n\x1a\n"
        + bloco(b"IHDR", struct.pack(">IIBBBBB", largura, altura, 8, 6, 0, 0, 0))
        + bloco(b"IDAT", zlib.compress(cru, 9))
        + bloco(b"IEND", b"")
    )


def png(linhas, paleta, escala):
    """PNG de `linhas` ampliado `escala` vezes, sem suavização."""
    transparente = b"\x00\x00\x00\x00"
    cru = []
    for linha in linhas:
        pixels = b"".join((rgba(paleta[c]) if c != "." else transparente) * escala for c in linha)
        cru += [pixels] * escala
    return codificar(LADO * escala, LADO * escala, cru)


def previa(desenhos, paleta, escala=4, colunas=11):
    """Os desenhos lado a lado, em fundo escuro em cima e claro embaixo."""
    margem = 8
    celula = LADO * escala + margem
    largura = colunas * celula + margem
    metade = -(-len(desenhos) // colunas) * celula + margem
    fundos = (rgba("#28282E"), rgba("#ECECF0"))
    imagem = [bytearray(fundos[y >= metade] * largura) for y in range(metade * 2)]
    for lado in (0, 1):
        for n, desenho in enumerate(desenhos):
            x0 = margem + (n % colunas) * celula
            y0 = lado * metade + margem + (n // colunas) * celula
            for y, linha in enumerate(desenho["linhas"]):
                for x, c in enumerate(linha):
                    if c == ".":
                        continue
                    inicio = (x0 + x * escala) * 4
                    for dy in range(escala):
                        imagem[y0 + y * escala + dy][inicio : inicio + escala * 4] = rgba(paleta[c]) * escala
    return codificar(largura, metade * 2, imagem)


def main():
    pacote = sys.argv[1] if len(sys.argv) > 1 else "retro-8bit"
    meta, paleta, desenhos = ler(AQUI / f"{pacote}.txt")
    escala = int(meta.pop("scale", "8"))
    destino = AQUI.parent / pacote
    (destino / "apps").mkdir(parents=True, exist_ok=True)
    for antigo in (destino / "apps").glob("*.png"):
        antigo.unlink()

    icones, reserva = {}, None
    for desenho in desenhos:
        arquivo = f"apps/{desenho['id']}.png"
        (destino / arquivo).write_bytes(png(desenho["linhas"], paleta, escala))
        for nome in desenho["nomes"]:
            if nome == "reserva":
                reserva = arquivo
            elif nome in icones:
                sys.exit(f"o nome {nome!r} está em @{desenho['id']} e em outro desenho")
            else:
                icones[nome] = arquivo

    manifesto = dict(meta)
    manifesto["icons"] = icones
    if reserva:
        manifesto["fallback"] = reserva
    texto = json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n"
    (destino / "pack.json").write_text(texto, encoding="utf-8", newline="\n")
    (AQUI / f"{pacote}-previa.png").write_bytes(previa(desenhos, paleta))
    print(f"{pacote}: {len(desenhos)} desenhos, {len(icones)} nomes")


if __name__ == "__main__":
    main()
