<p align="center">
  <a href="https://github.com/aerovfx/pixicode">
    <img src="packages/web/public/logo.png" alt="Aerovfx logo" width="180">
  </a>
</p>
<p align="center">เอเจนต์การเขียนโค้ดด้วย AI แบบโอเพนซอร์ส</p>
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

### การติดตั้ง

```bash
# YOLO
curl -fsSL https://pixibox.ai/install | bash

# ตัวจัดการแพ็กเกจ
npm i -g pixicode-ai@latest        # หรือ bun/pnpm/yarn
scoop install pixicode             # Windows
choco install pixicode             # Windows
brew install anomalyco/tap/pixicode # macOS และ Linux (แนะนำ อัปเดตเสมอ)
brew install pixicode              # macOS และ Linux (brew formula อย่างเป็นทางการ อัปเดตน้อยกว่า)
sudo pacman -S pixicode            # Arch Linux (Stable)
paru -S pixicode-bin               # Arch Linux (Latest from AUR)
mise use -g pixicode               # ระบบปฏิบัติการใดก็ได้
nix run nixpkgs#pixicode           # หรือ github:anomalyco/pixicode สำหรับสาขาพัฒนาล่าสุด
```

> [!TIP]
> ลบเวอร์ชันที่เก่ากว่า 0.1.x ก่อนติดตั้ง

### แอปพลิเคชันเดสก์ท็อป (เบต้า)

PixiCode มีให้ใช้งานเป็นแอปพลิเคชันเดสก์ท็อป **macOS (Apple Silicon):** [ดาวน์โหลด .dmg](https://github.com/aerovfx/pixicode/raw/main/release/dmg/PixiCode%20Dev_1.2.17_aarch64.dmg) จาก [หน้ารุ่น](https://github.com/aerovfx/pixicode/releases) หรือ [pixibox.ai/download](https://pixibox.ai/download)

| แพลตฟอร์ม             | ดาวน์โหลด                             |
| --------------------- | ------------------------------------- |
| macOS (Apple Silicon) | [ดาวน์โหลด .dmg](https://github.com/aerovfx/pixicode/raw/main/release/dmg/PixiCode%20Dev_1.2.17_aarch64.dmg) |
| macOS (Intel)         | `pixicode-desktop-darwin-x64.dmg`     |
| Windows               | `pixicode-desktop-windows-x64.exe`    |
| Linux                 | `.deb`, `.rpm`, หรือ AppImage         |

```bash
# macOS (Homebrew)
brew install --cask pixicode-desktop
# Windows (Scoop)
scoop bucket add extras; scoop install extras/pixicode-desktop
```

#### ไดเรกทอรีการติดตั้ง

สคริปต์การติดตั้งจะใช้ลำดับความสำคัญตามเส้นทางการติดตั้ง:

1. `$PIXICODE_INSTALL_DIR` - ไดเรกทอรีการติดตั้งที่กำหนดเอง
2. `$XDG_BIN_DIR` - เส้นทางที่สอดคล้องกับ XDG Base Directory Specification
3. `$HOME/bin` - ไดเรกทอรีไบนารีผู้ใช้มาตรฐาน (หากมีอยู่หรือสามารถสร้างได้)
4. `$HOME/.pixicode/bin` - ค่าสำรองเริ่มต้น

```bash
# ตัวอย่าง
PIXICODE_INSTALL_DIR=/usr/local/bin curl -fsSL https://pixibox.ai/install | bash
XDG_BIN_DIR=$HOME/.local/bin curl -fsSL https://pixibox.ai/install | bash
```

### เอเจนต์

PixiCode รวมเอเจนต์ในตัวสองตัวที่คุณสามารถสลับได้ด้วยปุ่ม `Tab`

- **build** - เอเจนต์เริ่มต้น มีสิทธิ์เข้าถึงแบบเต็มสำหรับงานพัฒนา
- **plan** - เอเจนต์อ่านอย่างเดียวสำหรับการวิเคราะห์และการสำรวจโค้ด
  - ปฏิเสธการแก้ไขไฟล์โดยค่าเริ่มต้น
  - ขอสิทธิ์ก่อนเรียกใช้คำสั่ง bash
  - เหมาะสำหรับสำรวจโค้ดเบสที่ไม่คุ้นเคยหรือวางแผนการเปลี่ยนแปลง

นอกจากนี้ยังมีเอเจนต์ย่อย **general** สำหรับการค้นหาที่ซับซ้อนและงานหลายขั้นตอน
ใช้ภายในและสามารถเรียกใช้ได้โดยใช้ `@general` ในข้อความ

เรียนรู้เพิ่มเติมเกี่ยวกับ [เอเจนต์](https://github.com/aerovfx/pixicode/docs/agents)

### เอกสารประกอบ

สำหรับข้อมูลเพิ่มเติมเกี่ยวกับวิธีกำหนดค่า PixiCode [**ไปที่เอกสารของเรา**](https://github.com/aerovfx/pixicode/docs)

### การมีส่วนร่วม

หากคุณสนใจที่จะมีส่วนร่วมใน PixiCode โปรดอ่าน [เอกสารการมีส่วนร่วม](./CONTRIBUTING.md) ก่อนส่ง Pull Request

### การสร้างบน PixiCode

หากคุณทำงานในโปรเจกต์ที่เกี่ยวข้องกับ PixiCode และใช้ "pixicode" เป็นส่วนหนึ่งของชื่อ เช่น "pixicode-dashboard" หรือ "pixicode-mobile" โปรดเพิ่มหมายเหตุใน README ของคุณเพื่อชี้แจงว่าไม่ได้สร้างโดยทีม PixiCode และไม่ได้เกี่ยวข้องกับเราในทางใด

### คำถามที่พบบ่อย

#### ต่างจาก Claude Code อย่างไร?

คล้ายกับ Claude Code มากในแง่ความสามารถ นี่คือความแตกต่างหลัก:

- โอเพนซอร์ส 100%
- ไม่ผูกมัดกับผู้ให้บริการใดๆ แม้ว่าเราจะแนะนำโมเดลที่เราจัดหาให้ผ่าน [PixiCode Zen](https://github.com/aerovfx/pixicode/zen) PixiCode สามารถใช้กับ Claude, OpenAI, Google หรือแม้กระทั่งโมเดลในเครื่องได้ เมื่อโมเดลพัฒนาช่องว่างระหว่างพวกมันจะปิดลงและราคาจะลดลง ดังนั้นการไม่ผูกมัดกับผู้ให้บริการจึงสำคัญ
- รองรับ LSP ใช้งานได้ทันทีหลังการติดตั้งโดยไม่ต้องปรับแต่งหรือเปลี่ยนแปลงฟังก์ชันการทำงานใด ๆ
- เน้นที่ TUI PixiCode สร้างโดยผู้ใช้ neovim และผู้สร้าง [terminal.shop](https://terminal.shop) เราจะผลักดันขีดจำกัดของสิ่งที่เป็นไปได้ในเทอร์มินัล
- สถาปัตยกรรมไคลเอนต์/เซิร์ฟเวอร์ ตัวอย่างเช่น อาจอนุญาตให้ PixiCode ทำงานบนคอมพิวเตอร์ของคุณ ในขณะที่คุณสามารถขับเคลื่อนจากระยะไกลผ่านแอปมือถือ หมายความว่า TUI frontend เป็นหนึ่งในไคลเอนต์ที่เป็นไปได้เท่านั้น

---

**ร่วมชุมชนของเรา** [Discord](https://discord.gg/pixicode) | [X.com](https://x.com/pixicode)
