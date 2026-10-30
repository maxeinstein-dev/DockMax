"""Gera uma skin de barra (spec 0020 §3) a partir do desenho em texto.

Uso: python gerar.py classic98

Lê `fonte/<id>.txt` e escreve `../<id>/skin.png` (o desenho em tamanho
natural: o dock o amplia até a altura da barra, sem suavização) e
`../<id>/skin.json` (o nome, o tamanho e o corte). Só usa a biblioteca
padrão; a saída é sempre a mesma para a mesma fonte.

O corte é o de 9 fatias, com as fatias de cima e de baixo vazias: a imagem
inteira vai na altura da barra, e só a largura muda. `left` e `right` são as
pontas, que ficam como estão; o meio se repete (`repeat`) ou estica
(`stretch`) até a largura da barra.
"""

import json
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent.parent / "icon-packs" / "fonte"))
from gerar import codificar, rgba  # noqa: E402


def ler(caminho):
    meta, paleta, linhas = {}, {}, []
    for numero, linha in enumerate(caminho.read_text(encoding="utf-8").splitlines(), 1):
        linha = linha.rstrip()
        if not linha or linha.startswith("#"):
            continue
        if linha.startswith("skin."):
            chave, valor = linha[5:].split(":", 1)
            meta[chave.strip()] = valor.strip()
        elif linha.startswith("cor."):
            chave, valor = linha[4:].split(":", 1)
            paleta[chave.strip()] = valor.strip()
        else:
            desconhecida = set(linha) - set(paleta) - {"."}
            if desconhecida:
                sys.exit(f"{caminho.name}:{numero}: cor fora da paleta: {''.join(sorted(desconhecida))}")
            linhas.append(linha)
    largura = len(linhas[0])
    if any(len(l) != largura for l in linhas):
        sys.exit(f"{caminho.name}: as linhas não têm todas {largura} caracteres")
    esquerda, direita = int(meta["left"]), int(meta["right"])
    if esquerda + direita >= largura:
        sys.exit(f"{caminho.name}: as pontas não deixam meio")
    if meta["middle"] not in ("repeat", "stretch"):
        sys.exit(f"{caminho.name}: middle é repeat ou stretch")
    return meta, paleta, linhas


def main():
    skin = sys.argv[1]
    meta, paleta, linhas = ler(AQUI / f"{skin}.txt")
    destino = AQUI.parent / skin
    destino.mkdir(exist_ok=True)
    cru = [b"".join(rgba(paleta[c]) if c != "." else b"\x00\x00\x00\x00" for c in linha) for linha in linhas]
    (destino / "skin.png").write_bytes(codificar(len(linhas[0]), len(linhas), cru))
    manifesto = {
        "name": meta["name"],
        "image": "skin.png",
        "width": len(linhas[0]),
        "height": len(linhas),
        "slice": {"top": 0, "right": int(meta["right"]), "bottom": 0, "left": int(meta["left"])},
        "middle": meta["middle"],
        "pixelated": True,
    }
    texto = json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n"
    (destino / "skin.json").write_text(texto, encoding="utf-8", newline="\n")
    print(f"{skin}: {len(linhas[0])}×{len(linhas)}")


if __name__ == "__main__":
    main()
