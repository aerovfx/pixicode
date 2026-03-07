<p align="center">
  <a href="https://github.com/aerovfx/pixicode">
    <img src="packages/web/public/logo.png" alt="Aerovfx logo" width="180">
  </a>
</p>
<p align="center">ওপেন সোর্স এআই কোডিং এজেন্ট।</p>
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

### ইনস্টলেশন (Installation)

```bash
# YOLO
curl -fsSL https://pixibox.ai/install | bash

# Package managers
npm i -g pixicode-ai@latest        # or bun/pnpm/yarn
scoop install pixicode             # Windows
choco install pixicode             # Windows
brew install anomalyco/tap/pixicode # macOS and Linux (recommended, always up to date)
brew install pixicode              # macOS and Linux (official brew formula, updated less)
sudo pacman -S pixicode            # Arch Linux (Stable)
paru -S pixicode-bin               # Arch Linux (Latest from AUR)
mise use -g pixicode               # Any OS
nix run nixpkgs#pixicode           # or github:anomalyco/pixicode for latest dev branch
```

> [!TIP]
> ইনস্টল করার আগে ০.১.x এর চেয়ে পুরোনো ভার্সনগুলো মুছে ফেলুন।

### ডেস্কটপ অ্যাপ (BETA)

PixiCode ডেস্কটপ অ্যাপ্লিকেশন হিসেবেও উপলব্ধ। **macOS (Apple Silicon):** [.dmg ডাউনলোড](https://github.com/aerovfx/pixicode/raw/main/release/dmg/PixiCode%20Dev_1.2.17_aarch64.dmg)। [রিলিজ পেজ](https://github.com/aerovfx/pixicode/releases) অথবা [pixibox.ai/download](https://pixibox.ai/download) থেকেও।

| প্ল্যাটফর্ম           | ডাউনলোড                               |
| --------------------- | ------------------------------------- |
| macOS (Apple Silicon) | [.dmg ডাউনলোড](https://github.com/aerovfx/pixicode/raw/main/release/dmg/PixiCode%20Dev_1.2.17_aarch64.dmg) |
| macOS (Intel)         | `pixicode-desktop-darwin-x64.dmg`     |
| Windows               | `pixicode-desktop-windows-x64.exe`    |
| Linux                 | `.deb`, `.rpm`, or AppImage           |

```bash
# macOS (Homebrew)
brew install --cask pixicode-desktop
# Windows (Scoop)
scoop bucket add extras; scoop install extras/pixicode-desktop
```

#### ইনস্টলেশন ডিরেক্টরি (Installation Directory)

ইনস্টল স্ক্রিপ্টটি ইনস্টলেশন পাতের জন্য নিম্নলিখিত অগ্রাধিকার ক্রম মেনে চলে:

1. `$PIXICODE_INSTALL_DIR` - কাস্টম ইনস্টলেশন ডিরেক্টরি
2. `$XDG_BIN_DIR` - XDG বেস ডিরেক্টরি স্পেসিফিকেশন সমর্থিত পাথ
3. `$HOME/bin` - সাধারণ ব্যবহারকারী বাইনারি ডিরেক্টরি (যদি বিদ্যমান থাকে বা তৈরি করা যায়)
4. `$HOME/.pixicode/bin` - ডিফল্ট ফলব্যাক

```bash
# উদাহরণ
PIXICODE_INSTALL_DIR=/usr/local/bin curl -fsSL https://pixibox.ai/install | bash
XDG_BIN_DIR=$HOME/.local/bin curl -fsSL https://pixibox.ai/install | bash
```

### এজেন্টস (Agents)

PixiCode এ দুটি বিল্ট-ইন এজেন্ট রয়েছে যা আপনি `Tab` কি(key) দিয়ে পরিবর্তন করতে পারবেন।

- **build** - ডিফল্ট, ডেভেলপমেন্টের কাজের জন্য সম্পূর্ণ অ্যাক্সেসযুক্ত এজেন্ট
- **plan** - বিশ্লেষণ এবং কোড এক্সপ্লোরেশনের জন্য রিড-ওনলি এজেন্ট
  - ডিফল্টভাবে ফাইল এডিট করতে দেয় না
  - ব্যাশ কমান্ড চালানোর আগে অনুমতি চায়
  - অপরিচিত কোডবেস এক্সপ্লোর করা বা পরিবর্তনের পরিকল্পনা করার জন্য আদর্শ

এছাড়াও জটিল অনুসন্ধান এবং মাল্টিস্টেপ টাস্কের জন্য একটি **general** সাবএজেন্ট অন্তর্ভুক্ত রয়েছে।
এটি অভ্যন্তরীণভাবে ব্যবহৃত হয় এবং মেসেজে `@general` লিখে ব্যবহার করা যেতে পারে।

এজেন্টদের সম্পর্কে আরও জানুন: [docs](https://github.com/aerovfx/pixicode/docs/agents)।

### ডকুমেন্টেশন (Documentation)

কিভাবে PixiCode কনফিগার করবেন সে সম্পর্কে আরও তথ্যের জন্য, [**আমাদের ডকস দেখুন**](https://github.com/aerovfx/pixicode/docs)।

### অবদান (Contributing)

আপনি যদি PixiCode এ অবদান রাখতে চান, অনুগ্রহ করে একটি পুল রিকোয়েস্ট সাবমিট করার আগে আমাদের [কন্ট্রিবিউটিং ডকস](./CONTRIBUTING.md) পড়ে নিন।

### PixiCode এর উপর বিল্ডিং (Building on PixiCode)

আপনি যদি এমন প্রজেক্টে কাজ করেন যা PixiCode এর সাথে সম্পর্কিত এবং প্রজেক্টের নামের অংশ হিসেবে "pixicode" ব্যবহার করেন, উদাহরণস্বরূপ "pixicode-dashboard" বা "pixicode-mobile", তবে দয়া করে আপনার README তে একটি নোট যোগ করে স্পষ্ট করুন যে এই প্রজেক্টটি PixiCode দল দ্বারা তৈরি হয়নি এবং আমাদের সাথে এর কোনো সরাসরি সম্পর্ক নেই।

### সচরাচর জিজ্ঞাসিত প্রশ্নাবলী (FAQ)

#### এটি ক্লড কোড (Claude Code) থেকে কীভাবে আলাদা?

ক্যাপাবিলিটির দিক থেকে এটি ক্লড কোডের (Claude Code) মতই। এখানে মূল পার্থক্যগুলো দেওয়া হলো:

- ১০০% ওপেন সোর্স
- কোনো প্রোভাইডারের সাথে আবদ্ধ নয়। যদিও আমরা [PixiCode Zen](https://github.com/aerovfx/pixicode/zen) এর মাধ্যমে মডেলসমূহ ব্যবহারের পরামর্শ দিই, PixiCode ক্লড (Claude), ওপেনএআই (OpenAI), গুগল (Google), অথবা লোকাল মডেলগুলোর সাথেও ব্যবহার করা যেতে পারে। যেমন যেমন মডেলগুলো উন্নত হবে, তাদের মধ্যকার পার্থক্য কমে আসবে এবং দামও কমবে, তাই প্রোভাইডার-অজ্ঞাস্টিক হওয়া খুবই গুরুত্বপূর্ণ।
- আউট-অফ-দ্য-বক্স LSP সাপোর্ট
- TUI এর উপর ফোকাস। PixiCode নিওভিম (neovim) ব্যবহারকারী এবং [terminal.shop](https://terminal.shop) এর নির্মাতাদের দ্বারা তৈরি; আমরা টার্মিনালে কী কী সম্ভব তার সীমাবদ্ধতা ছাড়িয়ে যাওয়ার চেষ্টা করছি।
- ক্লায়েন্ট/সার্ভার আর্কিটেকচার। এটি যেমন PixiCode কে আপনার কম্পিউটারে চালানোর সুযোগ দেয়, তেমনি আপনি মোবাইল অ্যাপ থেকে রিমোটলি এটি নিয়ন্ত্রণ করতে পারবেন, অর্থাৎ TUI ফ্রন্টএন্ড কেবল সম্ভাব্য ক্লায়েন্টগুলোর মধ্যে একটি।

---

**আমাদের কমিউনিটিতে যুক্ত হোন** [Discord](https://discord.gg/pixicode) | [X.com](https://x.com/pixicode)
