<p align="center">
  <a href="https://github.com/aerovfx/pixicode">
    <img src="packages/web/public/logo.png" alt="Aerovfx logo" width="180">
  </a>
</p>
<p align="center">AI-агент для програмування з відкритим кодом.</p>
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

### Встановлення

```bash
# YOLO
curl -fsSL https://pixibox.ai/install | bash

# Менеджери пакетів
npm i -g pixicode-ai@latest        # або bun/pnpm/yarn
scoop install pixicode             # Windows
choco install pixicode             # Windows
brew install anomalyco/tap/pixicode # macOS і Linux (рекомендовано, завжди актуально)
brew install pixicode              # macOS і Linux (офіційна формула Homebrew, оновлюється рідше)
sudo pacman -S pixicode            # Arch Linux (Stable)
paru -S pixicode-bin               # Arch Linux (Latest from AUR)
mise use -g pixicode               # Будь-яка ОС
nix run nixpkgs#pixicode           # або github:anomalyco/pixicode для найновішої dev-гілки
```

> [!TIP]
> Перед встановленням видаліть версії старші за 0.1.x.

### Десктопний застосунок (BETA)

PixiCode також доступний як десктопний застосунок. Завантажуйте напряму зі [сторінки релізів](https://github.com/anomalyco/pixicode/releases) або [pixibox.ai/download](https://github.com/aerovfx/pixicode/download).

| Платформа             | Завантаження                          |
| --------------------- | ------------------------------------- |
| macOS (Apple Silicon) | `pixicode-desktop-darwin-aarch64.dmg` |
| macOS (Intel)         | `pixicode-desktop-darwin-x64.dmg`     |
| Windows               | `pixicode-desktop-windows-x64.exe`    |
| Linux                 | `.deb`, `.rpm` або AppImage           |

```bash
# macOS (Homebrew)
brew install --cask pixicode-desktop
# Windows (Scoop)
scoop bucket add extras; scoop install extras/pixicode-desktop
```

#### Каталог встановлення

Скрипт встановлення дотримується такого порядку пріоритету для шляху встановлення:

1. `$PIXICODE_INSTALL_DIR` - Користувацький каталог встановлення
2. `$XDG_BIN_DIR` - Шлях, сумісний зі специфікацією XDG Base Directory
3. `$HOME/bin` - Стандартний каталог користувацьких бінарників (якщо існує або його можна створити)
4. `$HOME/.pixicode/bin` - Резервний варіант за замовчуванням

```bash
# Приклади
PIXICODE_INSTALL_DIR=/usr/local/bin curl -fsSL https://pixibox.ai/install | bash
XDG_BIN_DIR=$HOME/.local/bin curl -fsSL https://pixibox.ai/install | bash
```

### Агенти

PixiCode містить два вбудовані агенти, між якими можна перемикатися клавішею `Tab`.

- **build** - Агент за замовчуванням із повним доступом для завдань розробки
- **plan** - Агент лише для читання для аналізу та дослідження коду
  - За замовчуванням забороняє редагування файлів
  - Запитує дозвіл перед запуском bash-команд
  - Ідеально підходить для дослідження незнайомих кодових баз або планування змін

Також доступний допоміжний агент **general** для складного пошуку та багатокрокових завдань.
Він використовується всередині системи й може бути викликаний у повідомленнях через `@general`.

Дізнайтеся більше про [agents](https://github.com/aerovfx/pixicode/docs/agents).

### Документація

Щоб дізнатися більше про налаштування PixiCode, [**перейдіть до нашої документації**](https://github.com/aerovfx/pixicode/docs).

### Внесок

Якщо ви хочете зробити внесок в PixiCode, будь ласка, прочитайте нашу [документацію для контриб'юторів](./CONTRIBUTING.md) перед надсиланням pull request.

### Проєкти на базі PixiCode

Якщо ви працюєте над проєктом, пов'язаним з PixiCode, і використовуєте "pixicode" у назві, наприклад "pixicode-dashboard" або "pixicode-mobile", додайте примітку до свого README.
Уточніть, що цей проєкт не створений командою PixiCode і жодним чином не афілійований із нами.

### FAQ

#### Чим це відрізняється від Claude Code?

За можливостями це дуже схоже на Claude Code. Ось ключові відмінності:

- 100% open source
- Немає прив'язки до конкретного провайдера. Ми рекомендуємо моделі, які надаємо через [PixiCode Zen](https://github.com/aerovfx/pixicode/zen), але PixiCode також працює з Claude, OpenAI, Google і навіть локальними моделями. З розвитком моделей різниця між ними зменшуватиметься, а ціни падатимуть, тому незалежність від провайдера має значення.
- Підтримка LSP з коробки
- Фокус на TUI. PixiCode створено користувачами neovim та авторами [terminal.shop](https://terminal.shop); ми й надалі розширюватимемо межі можливого в терміналі.
- Клієнт-серверна архітектура. Наприклад, це дає змогу запускати PixiCode на вашому комп'ютері й керувати ним віддалено з мобільного застосунку, тобто TUI-фронтенд - лише один із можливих клієнтів.

---

**Приєднуйтеся до нашої спільноти** [Discord](https://discord.gg/pixicode) | [X.com](https://x.com/pixicode)
