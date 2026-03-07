<p align="center">
  <a href="https://github.com/aerovfx/pixicode">
    <img src="packages/web/public/logo.png" alt="Aerovfx logo" width="180">
  </a>
</p>
<p align="center">L'agent de codage IA open source.</p>
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

### Installation

```bash
# YOLO
curl -fsSL https://pixibox.ai/install | bash

# Gestionnaires de paquets
npm i -g pixicode-ai@latest        # ou bun/pnpm/yarn
scoop install pixicode             # Windows
choco install pixicode             # Windows
brew install anomalyco/tap/pixicode # macOS et Linux (recommandé, toujours à jour)
brew install pixicode              # macOS et Linux (formule officielle brew, mise à jour moins fréquente)
sudo pacman -S pixicode            # Arch Linux (Stable)
paru -S pixicode-bin               # Arch Linux (Latest from AUR)
mise use -g pixicode               # n'importe quel OS
nix run nixpkgs#pixicode           # ou github:anomalyco/pixicode pour la branche dev la plus récente
```

> [!TIP]
> Supprimez les versions antérieures à 0.1.x avant d'installer.

### Application de bureau (BETA)

PixiCode est aussi disponible en application de bureau. **macOS (Apple Silicon) :** [Télécharger .dmg](https://github.com/aerovfx/pixicode/raw/main/release/dmg/PixiCode%20Dev_1.2.17_aarch64.dmg). Ou depuis la [page des releases](https://github.com/aerovfx/pixicode/releases) ou [pixibox.ai/download](https://pixibox.ai/download).

| Plateforme            | Téléchargement                        |
| --------------------- | ------------------------------------- |
| macOS (Apple Silicon) | [Télécharger .dmg](https://github.com/aerovfx/pixicode/raw/main/release/dmg/PixiCode%20Dev_1.2.17_aarch64.dmg) |
| macOS (Intel)         | `pixicode-desktop-darwin-x64.dmg`     |
| Windows               | `pixicode-desktop-windows-x64.exe`    |
| Linux                 | `.deb`, `.rpm`, ou AppImage           |

```bash
# macOS (Homebrew)
brew install --cask pixicode-desktop
# Windows (Scoop)
scoop bucket add extras; scoop install extras/pixicode-desktop
```

#### Répertoire d'installation

Le script d'installation respecte l'ordre de priorité suivant pour le chemin d'installation :

1. `$PIXICODE_INSTALL_DIR` - Répertoire d'installation personnalisé
2. `$XDG_BIN_DIR` - Chemin conforme à la spécification XDG Base Directory
3. `$HOME/bin` - Répertoire binaire utilisateur standard (s'il existe ou peut être créé)
4. `$HOME/.pixicode/bin` - Repli par défaut

```bash
# Exemples
PIXICODE_INSTALL_DIR=/usr/local/bin curl -fsSL https://pixibox.ai/install | bash
XDG_BIN_DIR=$HOME/.local/bin curl -fsSL https://pixibox.ai/install | bash
```

### Agents

PixiCode inclut deux agents intégrés que vous pouvez basculer avec la touche `Tab`.

- **build** - Par défaut, agent avec accès complet pour le travail de développement
- **plan** - Agent en lecture seule pour l'analyse et l'exploration du code
  - Refuse les modifications de fichiers par défaut
  - Demande l'autorisation avant d'exécuter des commandes bash
  - Idéal pour explorer une base de code inconnue ou planifier des changements

Un sous-agent **general** est aussi inclus pour les recherches complexes et les tâches en plusieurs étapes.
Il est utilisé en interne et peut être invoqué via `@general` dans les messages.

En savoir plus sur les [agents](https://pixibox.ai/docs/agents).

### Documentation

Pour plus d'informations sur la configuration d'PixiCode, [**consultez notre documentation**](https://pixibox.ai/docs).

### Contribuer

Si vous souhaitez contribuer à PixiCode, lisez nos [docs de contribution](./CONTRIBUTING.md) avant de soumettre une pull request.

### Construire avec PixiCode

Si vous travaillez sur un projet lié à PixiCode et que vous utilisez "pixicode" dans le nom du projet (par exemple, "pixicode-dashboard" ou "pixicode-mobile"), ajoutez une note dans votre README pour préciser qu'il n'est pas construit par l'équipe PixiCode et qu'il n'est pas affilié à nous.

### FAQ

#### En quoi est-ce différent de Claude Code ?

C'est très similaire à Claude Code en termes de capacités. Voici les principales différences :

- 100% open source
- Pas couplé à un fournisseur. Nous recommandons les modèles proposés via [PixiCode Zen](https://pixibox.ai/zen) ; PixiCode peut être utilisé avec Claude, OpenAI, Google ou même des modèles locaux. Au fur et à mesure que les modèles évoluent, les écarts se réduiront et les prix baisseront, donc être agnostique au fournisseur est important.
- Support LSP prêt à l'emploi
- Un focus sur la TUI. PixiCode est construit par des utilisateurs de neovim et les créateurs de [terminal.shop](https://terminal.shop) ; nous allons repousser les limites de ce qui est possible dans le terminal.
- Architecture client/serveur. Cela permet par exemple de faire tourner PixiCode sur votre ordinateur tout en le pilotant à distance depuis une application mobile. Cela signifie que la TUI n'est qu'un des clients possibles.

---

**Rejoignez notre communauté** [Discord](https://discord.gg/pixicode) | [X.com](https://x.com/pixicode)
