<p align="center">
  <a href="https://github.com/aerovfx/pixicode">
    <img src="packages/web/public/logo.png" alt="Aerovfx logo" width="180">
  </a>
</p>
<p align="center">AI-kodeagent med åpen kildekode.</p>
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

### Installasjon

```bash
# YOLO
curl -fsSL https://pixibox.ai/install | bash

# Pakkehåndterere
npm i -g pixicode-ai@latest        # eller bun/pnpm/yarn
scoop install pixicode             # Windows
choco install pixicode             # Windows
brew install anomalyco/tap/pixicode # macOS og Linux (anbefalt, alltid oppdatert)
brew install pixicode              # macOS og Linux (offisiell brew-formel, oppdateres sjeldnere)
sudo pacman -S pixicode            # Arch Linux (Stable)
paru -S pixicode-bin               # Arch Linux (Latest from AUR)
mise use -g pixicode               # alle OS
nix run nixpkgs#pixicode           # eller github:anomalyco/pixicode for nyeste dev-branch
```

> [!TIP]
> Fjern versjoner eldre enn 0.1.x før du installerer.

### Desktop-app (BETA)

PixiCode er også tilgjengelig som en desktop-app. Last ned direkte fra [releases-siden](https://github.com/anomalyco/pixicode/releases) eller [pixibox.ai/download](https://github.com/aerovfx/pixicode/download).

| Plattform             | Nedlasting                            |
| --------------------- | ------------------------------------- |
| macOS (Apple Silicon) | `pixicode-desktop-darwin-aarch64.dmg` |
| macOS (Intel)         | `pixicode-desktop-darwin-x64.dmg`     |
| Windows               | `pixicode-desktop-windows-x64.exe`    |
| Linux                 | `.deb`, `.rpm` eller AppImage         |

```bash
# macOS (Homebrew)
brew install --cask pixicode-desktop
# Windows (Scoop)
scoop bucket add extras; scoop install extras/pixicode-desktop
```

#### Installasjonsmappe

Installasjonsskriptet bruker følgende prioritet for installasjonsstien:

1. `$PIXICODE_INSTALL_DIR` - Egendefinert installasjonsmappe
2. `$XDG_BIN_DIR` - Sti som følger XDG Base Directory Specification
3. `$HOME/bin` - Standard brukerbinar-mappe (hvis den finnes eller kan opprettes)
4. `$HOME/.pixicode/bin` - Standard fallback

```bash
# Eksempler
PIXICODE_INSTALL_DIR=/usr/local/bin curl -fsSL https://pixibox.ai/install | bash
XDG_BIN_DIR=$HOME/.local/bin curl -fsSL https://pixibox.ai/install | bash
```

### Agents

PixiCode har to innebygde agents du kan bytte mellom med `Tab`-tasten.

- **build** - Standard, agent med full tilgang for utviklingsarbeid
- **plan** - Skrivebeskyttet agent for analyse og kodeutforsking
  - Nekter filendringer som standard
  - Spør om tillatelse før bash-kommandoer
  - Ideell for å utforske ukjente kodebaser eller planlegge endringer

Det finnes også en **general**-subagent for komplekse søk og flertrinnsoppgaver.
Den brukes internt og kan kalles via `@general` i meldinger.

Les mer om [agents](https://github.com/aerovfx/pixicode/docs/agents).

### Dokumentasjon

For mer info om hvordan du konfigurerer PixiCode, [**se dokumentasjonen**](https://github.com/aerovfx/pixicode/docs).

### Bidra

Hvis du vil bidra til PixiCode, les [contributing docs](./CONTRIBUTING.md) før du sender en pull request.

### Bygge på PixiCode

Hvis du jobber med et prosjekt som er relatert til PixiCode og bruker "pixicode" som en del av navnet; for eksempel "pixicode-dashboard" eller "pixicode-mobile", legg inn en merknad i README som presiserer at det ikke er bygget av PixiCode-teamet og ikke er tilknyttet oss på noen måte.

### FAQ

#### Hvordan er dette forskjellig fra Claude Code?

Det er veldig likt Claude Code når det gjelder funksjonalitet. Her er de viktigste forskjellene:

- 100% open source
- Ikke knyttet til en bestemt leverandør. Selv om vi anbefaler modellene vi tilbyr gjennom [PixiCode Zen](https://github.com/aerovfx/pixicode/zen); kan PixiCode brukes med Claude, OpenAI, Google eller til og med lokale modeller. Etter hvert som modellene utvikler seg vil gapene lukkes og prisene gå ned, så det er viktig å være provider-agnostic.
- LSP-støtte rett ut av boksen
- Fokus på TUI. PixiCode er bygget av neovim-brukere og skaperne av [terminal.shop](https://terminal.shop); vi kommer til å presse grensene for hva som er mulig i terminalen.
- Klient/server-arkitektur. Dette kan for eksempel la PixiCode kjøre på maskinen din, mens du styrer den eksternt fra en mobilapp. Det betyr at TUI-frontend'en bare er en av de mulige klientene.

---

**Bli med i fellesskapet** [Discord](https://discord.gg/pixicode) | [X.com](https://x.com/pixicode)
