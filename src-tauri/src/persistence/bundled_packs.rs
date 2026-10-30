//! The icon packs that come with the app (spec 0019, G4): copied into the
//! packs folder the first time the app opens, as if the user had put them
//! there, so they show up in Configurações like any other pack and are never
//! turned on by themselves.
//!
//! A pack is copied again only when the app brings a new version of it: one
//! the user deleted stays deleted until then. What was copied is noted in
//! [`MARKER`], in the packs folder.

use std::fs;
use std::io;
use std::path::Path;

use crate::domain::icon_pack::Manifest;

/// The file, in the packs folder, with a line `<id> <version>` per pack
/// copied. Not a folder, so the scan leaves it out.
pub const MARKER: &str = ".embutidos";

/// A pack in the binary: its id and its files, by path inside the pack.
pub struct Bundled {
    pub id: &'static str,
    pub files: &'static [(&'static str, &'static [u8])],
}

macro_rules! pack_files {
    ($id:literal: $($path:literal),* $(,)?) => {
        &[$(($path, include_bytes!(concat!("../../icon-packs/", $id, "/", $path)))),*]
    };
}

/// Retrô 8-bit: pixel art made from `icon-packs/fonte/retro-8bit.txt` by
/// `icon-packs/fonte/gerar.py`. A test checks this list against the pack.
pub const RETRO_8BIT: Bundled = Bundled {
    id: "retro-8bit",
    files: pack_files!("retro-8bit":
        "pack.json",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/cartucho.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/edge.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/lixeira.png",
        "apps/notion.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Esmalte: the glyphs of `icon-packs/fonte/glifos.txt` in white on the
/// dock's glazed tile, drawn as SVG and rasterized by
/// `icon-packs/fonte/vetor.py`. A test checks this list against the pack.
pub const ESMALTE: Bundled = Bundled {
    id: "esmalte",
    files: pack_files!("esmalte":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Traço: the same glyphs alone, in a thin line, with an ink for the light
/// theme (`apps/`) and one for the dark (`escuro/`), drawn by
/// `icon-packs/fonte/vetor.py`. A test checks this list against the pack.
pub const TRACO: Bundled = Bundled {
    id: "traco",
    files: pack_files!("traco":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
        "escuro/amd.png",
        "escuro/battlenet.png",
        "escuro/bloco-de-notas.png",
        "escuro/brave.png",
        "escuro/calculadora.png",
        "escuro/captura.png",
        "escuro/chatgpt.png",
        "escuro/chrome.png",
        "escuro/claude.png",
        "escuro/configuracoes.png",
        "escuro/discord.png",
        "escuro/docker.png",
        "escuro/ea.png",
        "escuro/edge.png",
        "escuro/epic.png",
        "escuro/excel.png",
        "escuro/explorador.png",
        "escuro/faceit.png",
        "escuro/figma.png",
        "escuro/filezilla.png",
        "escuro/firefox.png",
        "escuro/fotos.png",
        "escuro/gerenciador.png",
        "escuro/github.png",
        "escuro/gog.png",
        "escuro/grade.png",
        "escuro/lixeira.png",
        "escuro/medal.png",
        "escuro/notion.png",
        "escuro/nvidia.png",
        "escuro/obs.png",
        "escuro/onenote.png",
        "escuro/opera.png",
        "escuro/outlook.png",
        "escuro/overwolf.png",
        "escuro/paint.png",
        "escuro/powerpoint.png",
        "escuro/powershell.png",
        "escuro/prompt.png",
        "escuro/riot.png",
        "escuro/slack.png",
        "escuro/spotify.png",
        "escuro/steam.png",
        "escuro/store.png",
        "escuro/teams.png",
        "escuro/telegram.png",
        "escuro/terminal.png",
        "escuro/twitch.png",
        "escuro/ubisoft.png",
        "escuro/vlc.png",
        "escuro/vscode.png",
        "escuro/whatsapp.png",
        "escuro/word.png",
        "escuro/xbox.png",
        "escuro/zoom.png",
    ),
};

/// Clássico 98: pixel art made from `icon-packs/fonte/classico-98.txt` by
/// `icon-packs/fonte/gerar.py`. A test checks this list against the pack.
pub const CLASSICO_98: Bundled = Bundled {
    id: "classico-98",
    files: pack_files!("classico-98":
        "pack.json",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/edge.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/notion.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Círculo: the glyphs in white on a flat disc in the app's color, drawn
/// by `icon-packs/fonte/vetor.py` from layers (`camadas.py`).
pub const CIRCULO: Bundled = Bundled {
    id: "circulo",
    files: pack_files!("circulo":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Contorno colorido: the glyphs alone, each in its app's hue, darker for
/// the light theme (`apps/`) and lighter for the dark (`escuro/`).
pub const CONTORNO: Bundled = Bundled {
    id: "contorno",
    files: pack_files!("contorno":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
        "escuro/amd.png",
        "escuro/battlenet.png",
        "escuro/bloco-de-notas.png",
        "escuro/brave.png",
        "escuro/calculadora.png",
        "escuro/captura.png",
        "escuro/chatgpt.png",
        "escuro/chrome.png",
        "escuro/claude.png",
        "escuro/configuracoes.png",
        "escuro/discord.png",
        "escuro/docker.png",
        "escuro/ea.png",
        "escuro/edge.png",
        "escuro/epic.png",
        "escuro/excel.png",
        "escuro/explorador.png",
        "escuro/faceit.png",
        "escuro/figma.png",
        "escuro/filezilla.png",
        "escuro/firefox.png",
        "escuro/fotos.png",
        "escuro/gerenciador.png",
        "escuro/github.png",
        "escuro/gog.png",
        "escuro/grade.png",
        "escuro/lixeira.png",
        "escuro/medal.png",
        "escuro/notion.png",
        "escuro/nvidia.png",
        "escuro/obs.png",
        "escuro/onenote.png",
        "escuro/opera.png",
        "escuro/outlook.png",
        "escuro/overwolf.png",
        "escuro/paint.png",
        "escuro/powerpoint.png",
        "escuro/powershell.png",
        "escuro/prompt.png",
        "escuro/riot.png",
        "escuro/slack.png",
        "escuro/spotify.png",
        "escuro/steam.png",
        "escuro/store.png",
        "escuro/teams.png",
        "escuro/telegram.png",
        "escuro/terminal.png",
        "escuro/twitch.png",
        "escuro/ubisoft.png",
        "escuro/vlc.png",
        "escuro/vscode.png",
        "escuro/whatsapp.png",
        "escuro/word.png",
        "escuro/xbox.png",
        "escuro/zoom.png",
    ),
};

/// Tom sobre tom: a tile and a glyph of the app's hue, a light tile in the
/// light theme and a dark one in the dark.
pub const TOM_SOBRE_TOM: Bundled = Bundled {
    id: "tom-sobre-tom",
    files: pack_files!("tom-sobre-tom":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
        "escuro/amd.png",
        "escuro/battlenet.png",
        "escuro/bloco-de-notas.png",
        "escuro/brave.png",
        "escuro/calculadora.png",
        "escuro/captura.png",
        "escuro/chatgpt.png",
        "escuro/chrome.png",
        "escuro/claude.png",
        "escuro/configuracoes.png",
        "escuro/discord.png",
        "escuro/docker.png",
        "escuro/ea.png",
        "escuro/edge.png",
        "escuro/epic.png",
        "escuro/excel.png",
        "escuro/explorador.png",
        "escuro/faceit.png",
        "escuro/figma.png",
        "escuro/filezilla.png",
        "escuro/firefox.png",
        "escuro/fotos.png",
        "escuro/gerenciador.png",
        "escuro/github.png",
        "escuro/gog.png",
        "escuro/grade.png",
        "escuro/lixeira.png",
        "escuro/medal.png",
        "escuro/notion.png",
        "escuro/nvidia.png",
        "escuro/obs.png",
        "escuro/onenote.png",
        "escuro/opera.png",
        "escuro/outlook.png",
        "escuro/overwolf.png",
        "escuro/paint.png",
        "escuro/powerpoint.png",
        "escuro/powershell.png",
        "escuro/prompt.png",
        "escuro/riot.png",
        "escuro/slack.png",
        "escuro/spotify.png",
        "escuro/steam.png",
        "escuro/store.png",
        "escuro/teams.png",
        "escuro/telegram.png",
        "escuro/terminal.png",
        "escuro/twitch.png",
        "escuro/ubisoft.png",
        "escuro/vlc.png",
        "escuro/vscode.png",
        "escuro/whatsapp.png",
        "escuro/word.png",
        "escuro/xbox.png",
        "escuro/zoom.png",
    ),
};

/// Vidro: a translucent tile tinted with the app's hue and a white glyph;
/// more opaque in the light theme.
pub const VIDRO: Bundled = Bundled {
    id: "vidro",
    files: pack_files!("vidro":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
        "escuro/amd.png",
        "escuro/battlenet.png",
        "escuro/bloco-de-notas.png",
        "escuro/brave.png",
        "escuro/calculadora.png",
        "escuro/captura.png",
        "escuro/chatgpt.png",
        "escuro/chrome.png",
        "escuro/claude.png",
        "escuro/configuracoes.png",
        "escuro/discord.png",
        "escuro/docker.png",
        "escuro/ea.png",
        "escuro/edge.png",
        "escuro/epic.png",
        "escuro/excel.png",
        "escuro/explorador.png",
        "escuro/faceit.png",
        "escuro/figma.png",
        "escuro/filezilla.png",
        "escuro/firefox.png",
        "escuro/fotos.png",
        "escuro/gerenciador.png",
        "escuro/github.png",
        "escuro/gog.png",
        "escuro/grade.png",
        "escuro/lixeira.png",
        "escuro/medal.png",
        "escuro/notion.png",
        "escuro/nvidia.png",
        "escuro/obs.png",
        "escuro/onenote.png",
        "escuro/opera.png",
        "escuro/outlook.png",
        "escuro/overwolf.png",
        "escuro/paint.png",
        "escuro/powerpoint.png",
        "escuro/powershell.png",
        "escuro/prompt.png",
        "escuro/riot.png",
        "escuro/slack.png",
        "escuro/spotify.png",
        "escuro/steam.png",
        "escuro/store.png",
        "escuro/teams.png",
        "escuro/telegram.png",
        "escuro/terminal.png",
        "escuro/twitch.png",
        "escuro/ubisoft.png",
        "escuro/vlc.png",
        "escuro/vscode.png",
        "escuro/whatsapp.png",
        "escuro/word.png",
        "escuro/xbox.png",
        "escuro/zoom.png",
    ),
};

/// Neon: the glyph as a neon tube of the app's hue, glowing; on a dark tile
/// in the light theme, loose in the dark.
pub const NEON: Bundled = Bundled {
    id: "neon",
    files: pack_files!("neon":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
        "escuro/amd.png",
        "escuro/battlenet.png",
        "escuro/bloco-de-notas.png",
        "escuro/brave.png",
        "escuro/calculadora.png",
        "escuro/captura.png",
        "escuro/chatgpt.png",
        "escuro/chrome.png",
        "escuro/claude.png",
        "escuro/configuracoes.png",
        "escuro/discord.png",
        "escuro/docker.png",
        "escuro/ea.png",
        "escuro/edge.png",
        "escuro/epic.png",
        "escuro/excel.png",
        "escuro/explorador.png",
        "escuro/faceit.png",
        "escuro/figma.png",
        "escuro/filezilla.png",
        "escuro/firefox.png",
        "escuro/fotos.png",
        "escuro/gerenciador.png",
        "escuro/github.png",
        "escuro/gog.png",
        "escuro/grade.png",
        "escuro/lixeira.png",
        "escuro/medal.png",
        "escuro/notion.png",
        "escuro/nvidia.png",
        "escuro/obs.png",
        "escuro/onenote.png",
        "escuro/opera.png",
        "escuro/outlook.png",
        "escuro/overwolf.png",
        "escuro/paint.png",
        "escuro/powerpoint.png",
        "escuro/powershell.png",
        "escuro/prompt.png",
        "escuro/riot.png",
        "escuro/slack.png",
        "escuro/spotify.png",
        "escuro/steam.png",
        "escuro/store.png",
        "escuro/teams.png",
        "escuro/telegram.png",
        "escuro/terminal.png",
        "escuro/twitch.png",
        "escuro/ubisoft.png",
        "escuro/vlc.png",
        "escuro/vscode.png",
        "escuro/whatsapp.png",
        "escuro/word.png",
        "escuro/xbox.png",
        "escuro/zoom.png",
    ),
};

/// Argila: a soft clay tile of the app's color, lit inside, with a white
/// glyph in relief.
pub const ARGILA: Bundled = Bundled {
    id: "argila",
    files: pack_files!("argila":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Fósforo: pixel art made from `icon-packs/fonte/fosforo.txt` by
/// `icon-packs/fonte/gerar.py`. A test checks this list against the pack.
pub const FOSFORO: Bundled = Bundled {
    id: "fosforo",
    files: pack_files!("fosforo":
        "pack.json",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/edge.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/grade.png",
        "apps/lixeira.png",
        "apps/notion.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Game Boy: pixel art made from `icon-packs/fonte/game-boy.txt` by
/// `icon-packs/fonte/gerar.py`. A test checks this list against the pack.
pub const GAME_BOY: Bundled = Bundled {
    id: "game-boy",
    files: pack_files!("game-boy":
        "pack.json",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/cartucho.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/edge.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/lixeira.png",
        "apps/notion.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Tático: the glyphs in sand on an octagonal olive plate, like a HUD; its
/// fallback is a reticle. Drawn by `icon-packs/fonte/vetor.py` from layers.
pub const TATICO: Bundled = Bundled {
    id: "tatico",
    files: pack_files!("tatico":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/mira.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Medieval 8-bit: pixel art made from `icon-packs/fonte/medieval.txt` by
/// `icon-packs/fonte/gerar.py`. A test checks this list against the pack.
pub const MEDIEVAL: Bundled = Bundled {
    id: "medieval",
    files: pack_files!("medieval":
        "pack.json",
        "apps/amd.png",
        "apps/battlenet.png",
        "apps/bau.png",
        "apps/bloco-de-notas.png",
        "apps/brave.png",
        "apps/calculadora.png",
        "apps/captura.png",
        "apps/chatgpt.png",
        "apps/chrome.png",
        "apps/claude.png",
        "apps/configuracoes.png",
        "apps/discord.png",
        "apps/docker.png",
        "apps/ea.png",
        "apps/edge.png",
        "apps/epic.png",
        "apps/excel.png",
        "apps/explorador.png",
        "apps/faceit.png",
        "apps/figma.png",
        "apps/filezilla.png",
        "apps/firefox.png",
        "apps/fotos.png",
        "apps/gerenciador.png",
        "apps/github.png",
        "apps/gog.png",
        "apps/lixeira.png",
        "apps/medal.png",
        "apps/notion.png",
        "apps/nvidia.png",
        "apps/obs.png",
        "apps/onenote.png",
        "apps/opera.png",
        "apps/outlook.png",
        "apps/overwolf.png",
        "apps/paint.png",
        "apps/powerpoint.png",
        "apps/powershell.png",
        "apps/prompt.png",
        "apps/riot.png",
        "apps/slack.png",
        "apps/spotify.png",
        "apps/steam.png",
        "apps/store.png",
        "apps/teams.png",
        "apps/telegram.png",
        "apps/terminal.png",
        "apps/twitch.png",
        "apps/ubisoft.png",
        "apps/vlc.png",
        "apps/vscode.png",
        "apps/whatsapp.png",
        "apps/word.png",
        "apps/xbox.png",
        "apps/zoom.png",
    ),
};

/// Every pack in the binary.
pub const ALL: &[Bundled] = &[
    RETRO_8BIT,
    ESMALTE,
    TRACO,
    CLASSICO_98,
    CIRCULO,
    CONTORNO,
    TOM_SOBRE_TOM,
    VIDRO,
    NEON,
    ARGILA,
    FOSFORO,
    GAME_BOY,
    TATICO,
    MEDIEVAL,
];

impl Bundled {
    /// The `pack.json` of the pack, as in the binary.
    pub fn manifest(&self) -> Manifest {
        let text = self
            .files
            .iter()
            .find(|(path, _)| *path == "pack.json")
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .unwrap_or_default();
        Manifest::parse(text).unwrap_or_default()
    }

    fn line(&self) -> String {
        format!("{} {}", self.id, self.manifest().version)
    }
}

/// Copies into `root` (the packs folder) each pack of `packs` not copied yet
/// in its version. Returns whether something was copied. A pack folder that
/// is a link is left alone.
pub fn install(root: &Path, packs: &[Bundled]) -> io::Result<bool> {
    let marker = root.join(MARKER);
    let mut noted = fs::read_to_string(&marker).unwrap_or_default();
    let mut copied = false;
    for pack in packs {
        let line = pack.line();
        if noted.lines().any(|noted| noted.trim() == line) {
            continue;
        }
        let dir = root.join(pack.id);
        if fs::symlink_metadata(&dir).is_ok_and(|meta| meta.file_type().is_symlink()) {
            continue;
        }
        for (path, bytes) in pack.files {
            let file = dir.join(path);
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file, bytes)?;
        }
        if !noted.is_empty() && !noted.ends_with('\n') {
            noted.push('\n');
        }
        noted.push_str(&line);
        noted.push('\n');
        fs::write(&marker, &noted)?;
        copied = true;
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::icon_pack::normalize_key;
    use crate::persistence::icon_packs;
    use std::collections::{BTreeMap, BTreeSet};
    use tempfile::TempDir;

    const SOURCE: &str = include_str!("../../icon-packs/fonte/retro-8bit.txt");
    const CLASSIC: &str = include_str!("../../icon-packs/fonte/classico-98.txt");
    const PHOSPHOR: &str = include_str!("../../icon-packs/fonte/fosforo.txt");
    const GAME_BOY_SOURCE: &str = include_str!("../../icon-packs/fonte/game-boy.txt");
    const MEDIEVAL_SOURCE: &str = include_str!("../../icon-packs/fonte/medieval.txt");
    const GLYPHS: &str = include_str!("../../icon-packs/fonte/glifos.txt");
    const LINE: &str = include_str!("../../icon-packs/fonte/traco.txt");
    const OUTLINE: &str = include_str!("../../icon-packs/fonte/contorno.txt");

    /// The folder of the repository a pack is made from.
    fn folder(path: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("icon-packs")
            .join(path)
    }

    /// The names of the files in `dir`, as `<prefix><name>`.
    fn names_in(dir: &str, prefix: &str) -> BTreeSet<String> {
        fs::read_dir(folder(dir))
            .unwrap()
            .map(|entry| format!("{prefix}{}", entry.unwrap().file_name().to_string_lossy()))
            .collect()
    }

    #[test]
    fn the_list_of_files_is_the_pack() {
        for (pack, name) in [
            (RETRO_8BIT, "Retrô 8-bit"),
            (ESMALTE, "Esmalte"),
            (TRACO, "Traço"),
            (CLASSICO_98, "Clássico 98"),
            (MEDIEVAL, "Medieval 8-bit"),
            (TATICO, "Tático"),
            (FOSFORO, "Fósforo"),
            (GAME_BOY, "Game Boy"),
            (CIRCULO, "Círculo"),
            (CONTORNO, "Contorno colorido"),
            (TOM_SOBRE_TOM, "Tom sobre tom"),
            (VIDRO, "Vidro"),
            (NEON, "Neon"),
            (ARGILA, "Argila"),
        ] {
            let manifest = pack.manifest();
            assert_eq!(manifest.name, name);
            let named: BTreeSet<&str> = manifest
                .icons
                .values()
                .chain(manifest.fallback.iter())
                .chain(manifest.dark.values())
                .map(String::as_str)
                .collect();
            // A dark image stands for one of the pack.
            assert!(
                manifest
                    .dark
                    .keys()
                    .all(|image| named.contains(image.as_str()))
            );
            let listed: BTreeSet<&str> = pack
                .files
                .iter()
                .map(|(path, _)| *path)
                .filter(|path| *path != "pack.json")
                .collect();
            assert_eq!(named, listed, "{name}");
            let mut on_disk = names_in(&format!("{}/apps", pack.id), "apps/");
            if folder(&format!("{}/escuro", pack.id)).is_dir() {
                on_disk.extend(names_in(&format!("{}/escuro", pack.id), "escuro/"));
            }
            assert_eq!(
                on_disk,
                listed.iter().map(|path| path.to_string()).collect(),
                "{name}"
            );
        }
    }

    #[test]
    fn the_installed_packs_are_ones_the_app_takes() {
        let root = TempDir::new().unwrap();
        assert!(install(root.path(), ALL).unwrap());
        let found = icon_packs::scan(root.path());
        assert_eq!(found.len(), ALL.len());
        for found in found {
            let pack = found.pack.clone().expect("the pack is valid");
            assert!(ALL.iter().any(|bundled| bundled.id == pack.id));
            for path in pack
                .manifest
                .icons
                .values()
                .chain(pack.manifest.fallback.iter())
                .chain(pack.manifest.dark.values())
            {
                assert!(icon_packs::read_image(&pack, path).is_some(), "{path}");
            }
            // The author's dock, by the names Windows shows.
            for app in [
                "Explorador de Arquivos",
                "FileZilla",
                "Google Chrome",
                "WhatsApp",
                "Terminal",
                "Visual Studio Code",
                "Bloco de notas",
            ] {
                assert!(pack.manifest.covers(&[app]), "{} {app}", pack.id);
            }
            let fallback = match pack.id.as_str() {
                "retro-8bit" | "game-boy" => "apps/cartucho.png",
                "tatico" => "apps/mira.png",
                "medieval" => "apps/bau.png",
                _ => "apps/grade.png",
            };
            assert_eq!(
                pack.manifest.image_for(&["Um programa qualquer"]),
                Some(fallback)
            );
        }
    }

    #[test]
    fn no_two_names_of_a_pack_are_one_key() {
        for pack in ALL {
            let manifest = pack.manifest();
            let mut seen = BTreeSet::new();
            for name in manifest.icons.keys() {
                assert!(seen.insert(normalize_key(name)), "{} {name}", pack.id);
            }
        }
    }

    #[test]
    fn a_pack_is_copied_once_per_version() {
        let root = TempDir::new().unwrap();
        let packs = &[RETRO_8BIT];
        assert!(install(root.path(), packs).unwrap());
        assert!(!install(root.path(), packs).unwrap());
        // Deleted by the user: it stays deleted.
        fs::remove_dir_all(root.path().join("retro-8bit")).unwrap();
        assert!(!install(root.path(), packs).unwrap());
        assert!(!root.path().join("retro-8bit").exists());
        // A new version comes back.
        fs::write(
            root.path().join(MARKER),
            "retro-8bit 0.9.0
",
        )
        .unwrap();
        assert!(install(root.path(), packs).unwrap());
        assert!(root.path().join("retro-8bit/pack.json").is_file());
        let marker = fs::read_to_string(root.path().join(MARKER)).unwrap();
        assert_eq!(
            marker,
            format!(
                "retro-8bit 0.9.0
{}
",
                RETRO_8BIT.line()
            )
        );
    }

    #[test]
    fn every_pack_is_noted_once() {
        let root = TempDir::new().unwrap();
        assert!(install(root.path(), ALL).unwrap());
        let marker = fs::read_to_string(root.path().join(MARKER)).unwrap();
        let lines: Vec<String> = ALL.iter().map(Bundled::line).collect();
        assert_eq!(
            marker,
            format!(
                "{}
",
                lines.join(
                    "
"
                )
            )
        );
    }

    /// What the source of a pixel art pack (`retro-8bit.txt`,
    /// `classico-98.txt`) says about the images.
    struct Source {
        /// A character of a drawing → its color.
        palette: BTreeMap<char, [u8; 3]>,
        /// The drawings by file: `grid` lines of `grid` characters each.
        drawings: BTreeMap<String, Vec<String>>,
        /// The side of a drawing, in characters.
        grid: usize,
        /// How many pixels of the PNG a character is, on each side.
        scale: usize,
    }

    fn source(text: &str) -> Source {
        let mut palette = BTreeMap::new();
        let mut drawings: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut scale = 0;
        let mut grid = 16;
        let mut current = String::new();
        for line in text.lines().map(str::trim_end) {
            if line.is_empty() || line.starts_with('#') || line.starts_with("nomes:") {
                continue;
            }
            if let Some(value) = line.strip_prefix("pack.scale:") {
                scale = value.trim().parse().unwrap();
                continue;
            }
            if let Some(value) = line.strip_prefix("pack.grid:") {
                grid = value.trim().parse().unwrap();
                continue;
            }
            if line.starts_with("pack.") {
                continue;
            }
            if let Some(color) = line.strip_prefix("cor.") {
                let (key, hex) = color.split_once(':').unwrap();
                let hex = hex.trim();
                let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).unwrap();
                palette.insert(
                    key.trim().chars().next().unwrap(),
                    [byte(1), byte(3), byte(5)],
                );
            } else if let Some(id) = line.strip_prefix('@') {
                current = format!("apps/{id}.png");
                drawings.insert(current.clone(), Vec::new());
            } else {
                drawings.get_mut(&current).unwrap().push(line.to_owned());
            }
        }
        Source {
            palette,
            drawings,
            grid,
            scale,
        }
    }

    #[test]
    fn the_images_are_the_drawings_of_the_source() {
        for (pack, text) in [
            (RETRO_8BIT, SOURCE),
            (CLASSICO_98, CLASSIC),
            (FOSFORO, PHOSPHOR),
            (GAME_BOY, GAME_BOY_SOURCE),
            (MEDIEVAL, MEDIEVAL_SOURCE),
        ] {
            pixels_are_drawings(pack, text);
        }
    }

    fn pixels_are_drawings(pack: Bundled, text: &str) {
        let Source {
            palette,
            drawings,
            grid,
            scale,
        } = source(text);
        assert_eq!(grid * scale, 128, "{}", pack.id);
        let images: BTreeMap<&str, &[u8]> = pack
            .files
            .iter()
            .filter(|(path, _)| *path != "pack.json")
            .copied()
            .collect();
        assert_eq!(
            drawings.keys().map(String::as_str).collect::<Vec<_>>(),
            images.keys().copied().collect::<Vec<_>>()
        );
        for (path, rows) in &drawings {
            let mut reader = png::Decoder::new(images[path.as_str()])
                .read_info()
                .unwrap();
            let mut pixels = vec![0; reader.output_buffer_size()];
            let frame = reader.next_frame(&mut pixels).unwrap();
            assert_eq!(frame.color_type, png::ColorType::Rgba, "{path}");
            assert_eq!(
                (frame.width, frame.height),
                ((grid * scale) as u32, (grid * scale) as u32)
            );
            assert_eq!(rows.len(), grid, "{path}");
            for (y, row) in rows.iter().enumerate() {
                assert_eq!(row.chars().count(), grid, "{path}");
                for (x, cell) in row.chars().enumerate() {
                    let expected = match cell {
                        '.' => [0, 0, 0, 0],
                        _ => {
                            let [r, g, b] = palette[&cell];
                            [r, g, b, 255]
                        }
                    };
                    // Every pixel of the cell's block is its color.
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let at = ((y * scale + dy) * grid * scale + x * scale + dx) * 4;
                            assert_eq!(pixels[at..at + 4], expected, "{path} ({x}, {y})");
                        }
                    }
                }
            }
        }
    }

    /// The glyphs of `glifos.txt`: the file of each and its glaze (hue,
    /// saturation, lightness).
    fn glyphs() -> BTreeMap<String, [f64; 3]> {
        let mut glyphs = BTreeMap::new();
        let mut current = String::new();
        for line in GLYPHS.lines().map(str::trim) {
            if let Some(id) = line.strip_prefix('@') {
                current = format!("apps/{id}.png");
            } else if let Some(glaze) = line.strip_prefix("esmalte:") {
                let values: Vec<f64> = glaze
                    .split_whitespace()
                    .map(|v| v.parse().unwrap())
                    .collect();
                glyphs.insert(current.clone(), [values[0], values[1], values[2]]);
            }
        }
        glyphs
    }

    /// `hsl(h s% l%)` as 8-bit RGB.
    fn hsl([h, s, l]: [f64; 3]) -> [f64; 3] {
        let (s, l) = (s / 100.0, l / 100.0);
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let (r, g, b) = match (h / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = l - c / 2.0;
        [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
    }

    fn decode(bytes: &[u8]) -> (u32, Vec<u8>) {
        let mut reader = png::Decoder::new(bytes).read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut pixels).unwrap();
        assert_eq!(frame.color_type, png::ColorType::Rgba);
        assert_eq!(frame.width, frame.height);
        (frame.width, pixels)
    }

    #[test]
    fn the_glazed_tiles_are_the_glyphs_of_the_source() {
        let glyphs = glyphs();
        let images: BTreeMap<&str, &[u8]> = ESMALTE
            .files
            .iter()
            .filter(|(path, _)| *path != "pack.json")
            .copied()
            .collect();
        assert_eq!(
            glyphs.keys().map(String::as_str).collect::<Vec<_>>(),
            images.keys().copied().collect::<Vec<_>>()
        );
        // The SVG each PNG was rasterized from is in the repository too.
        assert_eq!(
            names_in("fonte/esmalte-svg", "apps/")
                .iter()
                .map(|name| name.replace(".svg", ".png"))
                .collect::<Vec<_>>(),
            images
                .keys()
                .map(|path| path.to_string())
                .collect::<Vec<_>>()
        );
        for (path, glaze) in &glyphs {
            let (side, pixels) = decode(images[path.as_str()]);
            assert_eq!(side, 128, "{path}");
            let at = |x: u32, y: u32| {
                let i = ((y * side + x) * 4) as usize;
                [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
            };
            // Outside the superellipse, nothing.
            assert_eq!(at(0, 0), [0, 0, 0, 0], "{path}");
            assert_eq!(at(127, 0), [0, 0, 0, 0], "{path}");
            // On the tile (112 px from (8, 4)), left of the glyph and below
            // the sheen: the glaze of the source, between its two tones.
            let [r, g, b, a] = at(20, 64);
            assert_eq!(a, 255, "{path}");
            let [h, s, l] = *glaze;
            let f = (64.5 - 4.0) / 112.0;
            let (top, bottom) = (hsl([h, s, l + 7.0]), hsl([h, s, l - 5.0]));
            for (channel, (top, bottom)) in [r, g, b].into_iter().zip(top.into_iter().zip(bottom)) {
                let expected = top + (bottom - top) * f;
                assert!(
                    (f64::from(channel) - expected).abs() <= 2.0,
                    "{path}: {channel} for {expected}"
                );
            }
            // And the white glyph on it.
            let white = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| **p == [255; 4])
                .count();
            assert!(white > 100, "{path}: {white} white pixels");
        }
    }

    /// The relative luminance of an sRGB color (WCAG).
    fn luminance(rgb: [u8; 3]) -> f64 {
        let channel = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2])
    }

    fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn the_line_glyphs_have_an_ink_for_each_theme() {
        let ink = |key: &str| {
            let hex = LINE
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .unwrap()
                .trim();
            let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).unwrap();
            [byte(1), byte(3), byte(5)]
        };
        let manifest = TRACO.manifest();
        let images: BTreeMap<&str, &[u8]> = TRACO
            .files
            .iter()
            .filter(|(path, _)| *path != "pack.json")
            .copied()
            .collect();
        // The light ink on the dock's porcelain, the dark one on its
        // graphite: both read well past the 3:1 WCAG asks of graphics.
        for (folder_of, ink, surface) in [
            ("apps/", ink("cor:"), [247, 247, 249]),
            ("escuro/", ink("cor-escuro:"), [38, 39, 44]),
        ] {
            assert!(contrast(ink, surface) >= 4.5, "{folder_of}");
            let in_folder: Vec<&str> = images
                .keys()
                .copied()
                .filter(|path| path.starts_with(folder_of))
                .collect();
            assert_eq!(
                glyphs()
                    .keys()
                    .map(|path| path.replace("apps/", folder_of))
                    .collect::<Vec<_>>(),
                in_folder
            );
            for path in in_folder {
                let (side, pixels) = decode(images[path]);
                assert_eq!(side, 128, "{path}");
                let pixels = pixels.as_chunks::<4>().0;
                assert_eq!(pixels[0], [0, 0, 0, 0], "{path}");
                // Every pixel drawn is the ink; only its opacity varies.
                for pixel in pixels.iter().filter(|p| p[3] > 0) {
                    assert_eq!(pixel[..3], ink, "{path}");
                }
                let solid = pixels.iter().filter(|p| p[3] == 255).count();
                assert!(solid > 100, "{path}: {solid} solid pixels");
            }
        }
        // Every image has its dark one, and the SVGs of both are there.
        for (image, dark) in &manifest.dark {
            assert_eq!(dark, &image.replace("apps/", "escuro/"));
        }
        assert_eq!(manifest.dark.len(), glyphs().len());
        for svgs in ["fonte/traco-svg", "fonte/traco-escuro-svg"] {
            assert_eq!(
                names_in(svgs, "apps/")
                    .iter()
                    .map(|name| name.replace(".svg", ".png"))
                    .collect::<Vec<_>>(),
                glyphs().keys().cloned().collect::<Vec<_>>(),
                "{svgs}"
            );
        }
    }

    #[test]
    fn every_classic_icon_is_a_98_button() {
        let Source { drawings, .. } = source(CLASSIC);
        for (path, rows) in drawings {
            let row = |y: usize| rows[y].as_bytes();
            let last = rows.len() - 1;
            // Light on the top and left edges, shadow on the bottom and
            // right ones: the 98 bevel, in white, light gray, gray, black.
            for i in 0..last {
                assert_eq!((row(0)[i], row(i)[0]), (b'f', b'f'), "{path} {i}");
            }
            for i in 1..last - 1 {
                assert_eq!((row(1)[i], row(i)[1]), (b'g', b'g'), "{path} {i}");
            }
            for i in 1..=last {
                let shade = if i == last { b'0' } else { b'8' };
                assert_eq!(
                    (row(last - 1)[i], row(i)[last - 1]),
                    (shade, shade),
                    "{path} {i}"
                );
            }
            for i in 0..=last {
                assert_eq!((row(last)[i], row(i)[last]), (b'0', b'0'), "{path} {i}");
            }
        }
    }

    /// The PNGs of `pack` in `folder` (`apps/` or `escuro/`), by path.
    fn images_in(pack: &Bundled, folder: &str) -> BTreeMap<&'static str, &'static [u8]> {
        pack.files
            .iter()
            .filter(|(path, _)| path.starts_with(folder))
            .copied()
            .collect()
    }

    #[test]
    fn every_vector_pack_draws_every_glyph_in_each_of_its_themes() {
        for (pack, themes) in [
            (CIRCULO, &["apps/"][..]),
            (CONTORNO, &["apps/", "escuro/"][..]),
            (TOM_SOBRE_TOM, &["apps/", "escuro/"][..]),
            (VIDRO, &["apps/", "escuro/"][..]),
            (NEON, &["apps/", "escuro/"][..]),
            (ARGILA, &["apps/"][..]),
            (TATICO, &["apps/"][..]),
        ] {
            let manifest = pack.manifest();
            let svg_folder = |theme: &str| match theme {
                "apps/" => format!("fonte/{}-svg", pack.id),
                _ => format!("fonte/{}-escuro-svg", pack.id),
            };
            for theme in themes {
                let images = images_in(&pack, theme);
                // Tático has its own fallback: a reticle instead of the grid.
                let mut expected: Vec<String> = glyphs()
                    .keys()
                    .map(|path| path.replace("apps/", theme))
                    .map(|path| match pack.id {
                        "tatico" => path.replace("grade.png", "mira.png"),
                        _ => path,
                    })
                    .collect();
                expected.sort();
                assert_eq!(
                    images
                        .keys()
                        .map(|path| path.to_string())
                        .collect::<Vec<_>>(),
                    expected,
                    "{} {theme}",
                    pack.id
                );
                // The SVG each PNG was drawn from is in the repository too.
                assert_eq!(
                    names_in(&svg_folder(theme), theme)
                        .iter()
                        .map(|name| name.replace(".svg", ".png"))
                        .collect::<Vec<_>>(),
                    expected,
                    "{} {theme}",
                    pack.id
                );
                for (path, bytes) in images {
                    let (side, pixels) = decode(bytes);
                    assert_eq!(side, 128, "{} {path}", pack.id);
                    let pixels = pixels.as_chunks::<4>().0;
                    // A corner is free (the shapes keep a margin) and the
                    // middle has something drawn.
                    assert!(pixels[0][3] <= 8, "{} {path}", pack.id);
                    let drawn = pixels.iter().filter(|p| p[3] == 255).count();
                    assert!(drawn > 100, "{} {path}: {drawn}", pack.id);
                }
            }
            // A pack with a dark theme has a dark image for every image.
            if themes.len() == 2 {
                assert_eq!(manifest.dark.len(), glyphs().len(), "{}", pack.id);
            } else {
                assert!(manifest.dark.is_empty(), "{}", pack.id);
            }
        }
    }

    #[test]
    fn the_outline_glyphs_take_their_apps_hue_and_read_on_both_themes() {
        let value = |key: &str| -> f64 {
            OUTLINE
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .unwrap()
                .trim()
                .parse()
                .unwrap()
        };
        let saturation = value("saturacao:");
        for (folder_of, lightness, surface) in [
            ("apps/", value("luz:"), [247, 247, 249]),
            ("escuro/", value("luz-escuro:"), [38, 39, 44]),
        ] {
            let images = images_in(&CONTORNO, folder_of);
            for (path, [hue, _, _]) in glyphs() {
                let rgb = hsl([hue, saturation, lightness]);
                let ink = rgb.map(|c| c.round() as u8);
                // Every hue reads on the dock's surface of its theme.
                assert!(
                    contrast(ink, surface) >= 3.0,
                    "{path} {folder_of}: {:.2}",
                    contrast(ink, surface)
                );
                let (_, pixels) = decode(images[path.replace("apps/", folder_of).as_str()]);
                for pixel in pixels.as_chunks::<4>().0.iter().filter(|p| p[3] > 0) {
                    for (got, want) in pixel[..3].iter().zip(ink) {
                        assert!(got.abs_diff(want) <= 1, "{path} {folder_of}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_game_boy_has_its_four_greens_and_the_phosphor_its_screen() {
        let Source { palette, .. } = source(GAME_BOY_SOURCE);
        assert_eq!(palette.len(), 4);
        // From dark to light, every green lighter than the one before.
        let lights: Vec<f64> = ['0', '1', '2', '3']
            .iter()
            .map(|c| luminance(palette[c]))
            .collect();
        assert!(
            lights.windows(2).all(|pair| pair[1] > pair[0] * 1.8),
            "{lights:?}"
        );
        let Source { drawings, .. } = source(PHOSPHOR);
        for (path, rows) in drawings {
            // The screen, inside the bezel, is scan lines or lit phosphor.
            for row in &rows[3..29] {
                for cell in row[3..29].chars() {
                    assert!("01234".contains(cell), "{path}");
                }
            }
            assert!(rows.iter().any(|row| row.contains('4')), "{path}");
        }
    }

    #[test]
    fn the_vector_packs_cover_the_player_launchers() {
        // By the names Windows shows (spec 0020, Q4).
        let launchers = [
            "Epic Games Launcher",
            "Battle.net",
            "Riot Client",
            "EA app",
            "Ubisoft Connect",
            "GOG Galaxy",
            "FACEIT",
            "NVIDIA App",
            "AMD Software: Adrenalin Edition",
            "Overwolf",
            "Twitch Studio",
            "Medal",
        ];
        for pack in [
            ESMALTE,
            TRACO,
            CIRCULO,
            CONTORNO,
            TOM_SOBRE_TOM,
            VIDRO,
            NEON,
            ARGILA,
            TATICO,
            MEDIEVAL,
        ] {
            let manifest = pack.manifest();
            for app in launchers {
                assert!(manifest.covers(&[app]), "{} {app}", pack.id);
            }
        }
    }
}
