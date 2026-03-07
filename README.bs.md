<p align="center">
  <a href="https://github.com/aerovfx/pixicode">
    <img src="packages/web/public/logo.png" alt="Aerovfx logo" width="180">
  </a>
</p>
<p align="center">PixiCode je open source AI agent za programiranje.</p>
<p align="center">
  <a href="https://github.com/aerovfx/pixicode"><img alt="Repo" src="https://img.shields.io/badge/repo-aerovfx%2Fpixicode-blue?style=flat-square" /></a>
</p>

<p align="center">
  <a href="README.md">English</a> |
  <a href="README.zh.md">简体中文</a> |
  <a href="README.zht.md">繁體中文</a> |
  <a href="README.ko.md">한국어</a> |
  <a href="README.de.md">Deutsch</a> |
  <a href="README.es.md">Español</a> |
  <a href="README.fr.md">Français</a> |
  <a href="README.it.md">Italiano</a> |
  <a href="README.da.md">Dansk</a> |
  <a href="README.ja.md">日本語</a> |
  <a href="README.pl.md">Polski</a> |
  <a href="README.ru.md">Русский</a> |
  <a href="README.bs.md">Bosanski</a> |
  <a href="README.ar.md">العربية</a> |
  <a href="README.no.md">Norsk</a> |
  <a href="README.br.md">Português (Brasil)</a> |
  <a href="README.th.md">ไทย</a> |
  <a href="README.tr.md">Türkçe</a> |
  <a href="README.uk.md">Українська</a> |
  <a href="README.bn.md">বাংলা</a> |
  <a href="README.gr.md">Ελληνικά</a>
</p>

[![Aerovfx — session interface: session + editor + file tree](docs/interface-session.png)](docs/interface-session.png)

---

### Instalacija

```bash
# YOLO
curl -fsSL https://pixibox.ai/install | bash

# Package manageri
npm i -g pixicode-ai@latest        # ili bun/pnpm/yarn
scoop install pixicode             # Windows
choco install pixicode             # Windows
brew install anomalyco/tap/pixicode # macOS i Linux (preporučeno, uvijek ažurno)
brew install pixicode              # macOS i Linux (zvanična brew formula, rjeđe se ažurira)
sudo pacman -S pixicode            # Arch Linux (Stable)
paru -S pixicode-bin               # Arch Linux (Latest from AUR)
mise use -g pixicode               # Bilo koji OS
nix run nixpkgs#pixicode           # ili github:anomalyco/pixicode za najnoviji dev branch
```

> [!TIP]
> Ukloni verzije starije od 0.1.x prije instalacije.

### Desktop aplikacija (BETA)

PixiCode je dostupan i kao desktop aplikacija. Preuzmi je direktno sa [stranice izdanja](https://github.com/anomalyco/pixicode/releases) ili sa [pixibox.ai/download](https://github.com/aerovfx/pixicode/download).

| Platforma             | Preuzimanje                           |
| --------------------- | ------------------------------------- |
| macOS (Apple Silicon) | `pixicode-desktop-darwin-aarch64.dmg` |
| macOS (Intel)         | `pixicode-desktop-darwin-x64.dmg`     |
| Windows               | `pixicode-desktop-windows-x64.exe`    |
| Linux                 | `.deb`, `.rpm`, ili AppImage          |

```bash
# macOS (Homebrew)
brew install --cask pixicode-desktop
# Windows (Scoop)
scoop bucket add extras; scoop install extras/pixicode-desktop
```

#### Instalacijski direktorij

Instalacijska skripta koristi sljedeći redoslijed prioriteta za putanju instalacije:

1. `$PIXICODE_INSTALL_DIR` - Prilagođeni instalacijski direktorij
2. `$XDG_BIN_DIR` - Putanja usklađena sa XDG Base Directory specifikacijom
3. `$HOME/bin` - Standardni korisnički bin direktorij (ako postoji ili se može kreirati)
4. `$HOME/.pixicode/bin` - Podrazumijevana rezervna lokacija

```bash
# Primjeri
PIXICODE_INSTALL_DIR=/usr/local/bin curl -fsSL https://pixibox.ai/install | bash
XDG_BIN_DIR=$HOME/.local/bin curl -fsSL https://pixibox.ai/install | bash
```

### Agenti

PixiCode uključuje dva ugrađena agenta između kojih možeš prebacivati tasterom `Tab`.

- **build** - Podrazumijevani agent sa punim pristupom za razvoj
- **plan** - Agent samo za čitanje za analizu i istraživanje koda
  - Podrazumijevano zabranjuje izmjene datoteka
  - Traži dozvolu prije pokretanja bash komandi
  - Idealan za istraživanje nepoznatih codebase-ova ili planiranje izmjena

Uključen je i **general** pod-agent za složene pretrage i višekoračne zadatke.
Koristi se interno i može se pozvati pomoću `@general` u porukama.

Saznaj više o [agentima](https://github.com/aerovfx/pixicode/docs/agents).

### Dokumentacija

Za više informacija o konfiguraciji PixiCode-a, [**pogledaj dokumentaciju**](https://github.com/aerovfx/pixicode/docs).

### Doprinosi

Ako želiš doprinositi PixiCode-u, pročitaj [upute za doprinošenje](./CONTRIBUTING.md) prije slanja pull requesta.

### Gradnja na PixiCode-u

Ako radiš na projektu koji je povezan s PixiCode-om i koristi "pixicode" kao dio naziva, npr. "pixicode-dashboard" ili "pixicode-mobile", dodaj napomenu u svoj README da projekat nije napravio PixiCode tim i da nije povezan s nama.

### FAQ

#### Po čemu se razlikuje od Claude Code-a?

Po mogućnostima je vrlo sličan Claude Code-u. Ključne razlike su:

- 100% open source
- Nije vezan za jednog provajdera. Iako preporučujemo modele koje nudimo kroz [PixiCode Zen](https://github.com/aerovfx/pixicode/zen), PixiCode možeš koristiti s Claude, OpenAI, Google ili čak lokalnim modelima. Kako modeli napreduju, razlike među njima će se smanjivati, a cijene padati, zato je nezavisnost od provajdera važna.
- LSP podrška odmah po instalaciji
- Fokus na TUI. PixiCode grade neovim korisnici i kreatori [terminal.shop](https://terminal.shop); pomjeraćemo granice onoga što je moguće u terminalu.
- Klijent/server arhitektura. To, recimo, omogućava da PixiCode radi na tvom računaru dok ga daljinski koristiš iz mobilne aplikacije, što znači da je TUI frontend samo jedan od mogućih klijenata.

---

**Pridruži se našoj zajednici** [Discord](https://discord.gg/pixicode) | [X.com](https://x.com/pixicode)
