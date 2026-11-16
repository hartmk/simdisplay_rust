# Apex Display · Rust

แอปเดสก์ท็อป sim racing เขียนด้วย Rust ทั้ง UI และระบบเบื้องหลัง ใช้ egui/eframe ไม่มี Electron, JavaScript หรือ WebView ใน runtime

![Rust dashboard studio](docs/studio.png)

## เปิดใช้งาน

ติดตั้ง Rust stable ผ่าน [rustup](https://rustup.rs/) แล้วรันในโฟลเดอร์นี้:

```sh
cargo run --release --locked
```

Windows ต้องมี Visual Studio C++ Build Tools; macOS ต้องมี Xcode Command Line Tools (`xcode-select --install`)

macOS: สร้างไฟล์แอปด้วย `sh scripts/package-macos.sh` แล้วเปิด `release/Apex Display Rust.app` หรือใช้ ZIP ที่สคริปต์สร้าง แอปใช้ ad-hoc signing ในเครื่อง ไม่มี Developer ID/notarization

Windows แบบพร้อมใช้: แตกไฟล์ `release/Apex-Display-Rust-Windows-x64.zip` แล้วเปิด `simdisplay.exe` หรือคัดลอกไฟล์จาก `release/windows-x64/` ไปยังเครื่อง Windows x64 ไม่ต้องติดตั้ง Rust หรือ Node.js

Build บน Windows: เปิด Developer PowerShell ของ Visual Studio (พร้อม Windows SDK เพื่อเรียก `rc.exe`) แล้วรัน `cargo build --release --locked` แล้วเปิด `target/release/simdisplay.exe` มี workflow `Rust desktop builds` แบบ manual สำหรับสร้าง ZIP บน Windows/macOS โดยไม่ publish

Cross-build จาก macOS/Linux: ติดตั้ง LLVM/LLD และ [cargo-xwin](https://github.com/rust-cross/cargo-xwin) ให้เรียกได้จาก `PATH` จากนั้น:

```sh
rustup target add x86_64-pc-windows-msvc
sh scripts/package-windows.sh
```

สคริปต์สร้าง Windows x64 ด้วย MSVC target และ static C runtime แล้วแพ็ก `.exe`, คู่มือ และใบอนุญาตฟอนต์เป็น ZIP การ cross-build สำเร็จไม่ใช่การยืนยันว่าได้ทดสอบ runtime บน Windows แล้ว

## การใช้งาน

- เลือกวิดเจ็ตจากแถบซ้าย (สูงสุด 100) คลิกบน canvas เพื่อเลือก แล้วปรับที่ Inspector
- ลากชื่อเพื่อย้าย ลากมุมล่างขวาเพื่อปรับขนาด Grid snap ทีละ 8 px; คลิกมุมแล้วใช้ลูกศรเพื่อปรับขนาดด้วยคีย์บอร์ด
- ลูกศรขยับ 1 px; Shift + ลูกศร 10 px; Ctrl/Cmd + Z ย้อนกลับ และ Ctrl/Cmd + Shift + Z ทำซ้ำ (60 ขั้น)
- Ctrl/Cmd + D ทำสำเนา; Delete ลบ; ปุ่มชั้นบน/ล่างจัดลำดับซ้อน; ล็อกเพื่อป้องกันการย้าย/ย่อขยาย
- ปรับสีตัวอักษร พื้นหลัง ความโปร่งใส ฟอนต์ ขนาด และค่าเตือน ตัวเลขเวลารอบใช้มิลลิวินาที
- RPM ตั้งได้ 1–12 ช่วงสี เริ่มที่ 0 เรียงเพิ่ม ใช้หน่วย RPM หรือ % ของรอบสูงสุด มีค่าสำรองเมื่อเกมไม่ส่ง max RPM ปรับแล้วบันทึกอัตโนมัติ
- Canvas รองรับ Full HD, 1920 × 480, 480 × 1920, QHD และกำหนดเอง เปลี่ยนขนาดด้วย “ใช้ขนาดนี้”; Undo ได้
- บันทึกอัตโนมัติลงเครื่อง; “เก็บเลย์เอาต์” เก็บหลายแบบ ชื่อซ้ำจะอัปเดต
- Import/Export JSON ใช้รูปแบบ version 1 ร่วมกับแอป Electron เดิม เลย์เอาต์เก่าได้รับค่าเริ่มต้นอัตโนมัติ
- Preview ซ่อนเครื่องมือแก้ไข เลือกจอแล้วกด “เปิดบนจอแข่ง” เพื่อย้ายหน้าต่างไปเต็มจอ กด Escape กลับ editor

วิดเจ็ตทั้งหมด 19 แบบ: RPM number, Gear, Speed, Engine RPM, Current lap, Best lap, Last lap, Lap times (last/best/current), Tyres, Fuel, Position, Lap, Throttle, Brake, DRS active, Traction control, ABS, Session remaining และ Custom text

Best lap และ Last lap เป็นวิดเจ็ตเวลาแยก แสดง `นาที:วินาที.มิลลิวินาที` และ `—` เมื่อไม่มีข้อมูล ใช้เกณฑ์สีเตือนหน่วยมิลลิวินาทีได้ เลย์เอาต์ที่มีวิดเจ็ตใหม่สองแบบนี้ใช้กับแอป Rust; แอป Electron รุ่นเดิมยังไม่รู้จักชนิดวิดเจ็ตใหม่

วิดเจ็ตสถานะใหม่:

- **DRS active**: `ACTIVE` เมื่อ DRS เปิดจริง และ `OFF` เมื่อปิด อ่านจาก Car Telemetry ไม่ใช้ค่าอนุญาตเปิด DRS
- **Traction control**: `OFF`, `MEDIUM`, `FULL` ตามระดับระบบช่วยขับ
- **ABS**: `OFF`, `ON` ตามการตั้งค่าระบบช่วยเบรก

Traction/ABS เป็นการตั้งค่าระบบช่วยขับ ไม่ใช่การตรวจจับจังหวะที่ระบบแทรกแซงล้อ ตาม [สเปก EA F1 25 v3](https://forums.ea.com/t5/s/tghpe58374/attachments/tghpe58374/f1-games-game-info-hub-en/61/4/Data%20Output%20from%20F1%2025%20v3.pdf) หน้า 10–11 ค่าที่ไม่รองรับ ขาดหาย หรือหมดอายุ แสดง `—` แยกจาก `OFF` มีข้อมูล Demo สำหรับลองวางวิดเจ็ต และเลย์เอาต์ที่มีชนิดใหม่ใช้กับแอป Rust

ยางเรียง FL/FR/RL/RR: เขียวอุณหภูมิผิวยาง °C, เหลืองแรงดัน PSI, ฟ้าอุณหภูมิเบรก °C, ขาวเปอร์เซ็นต์สึก; 0% หมายถึงยังไม่สึก ค่าที่ไม่มีแสดง `—`

## Telemetry

Demo เปิดได้โดยไม่ใช้เกมและมีป้าย DEMO เสมอ

F1 25: เปิด UDP Telemetry ที่ `127.0.0.1:20777`, format **2025**, packet version 1, telemetry Public แล้วเลือก F1 25 ในแอป รองรับ Session, Lap Data, Car Telemetry, Car Status, Car Damage และ Session History ของรถผู้เล่นหลัก

ข้อมูลแต่ละค่าหมดอายุใน 2.5 วินาที ไม่มีข้อมูลจะแสดง WAITING; พอร์ตถูกใช้งานแสดง ERROR และกดเชื่อมต่อใหม่ได้ F1 fuel ใช้ kg, Demo ใช้ L; best lap ต้องเป็นรอบ valid

AC, ACC, LMU และ iRacing ยังไม่มี receiver และแสดง NOT READY ตามขอบเขตแอปเดิม

## ข้อมูลและการกู้คืน

เก็บ `workspace.json` แยกจาก Electron ใน application data directory ของ SimdisplayRust:

- macOS: `~/Library/Application Support/local.Apex.SimdisplayRust/`
- Windows: `%APPDATA%\Apex\SimdisplayRust\data\`

ระบุโฟลเดอร์อื่นได้ด้วย `simdisplay --data-dir /path/to/folder` ไม่ย้าย Electron localStorage อัตโนมัติ ให้ Export/Import JSON แทน

หากไฟล์เสีย แอปหยุด autosave เพื่อรักษาไฟล์เดิม ปุ่ม “สำรองไฟล์เดิม แล้วเริ่มบันทึกใหม่” จะคัดลอกไฟล์เดิมเป็น `.backup-<uuid>.json` ก่อนอนุญาตให้เขียนใหม่ การบันทึกใช้ temporary file และ replace หลังเขียนเสร็จ

สร้าง default layout JSON โดยไม่เปิด UI: `simdisplay --export-default /path/to/dashboard.json`

## ตรวจสอบ

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

ชุดทดสอบครอบคลุม JSON เดิม/เก่า, validation, geometry/RPM/history, editor interactions ผ่าน egui, packet สังเคราะห์, per-field expiry, UDP loopback และ storage recovery

สถานะการตรวจ native UI และแพ็กเกจจะบันทึกใน `docs/verification.md` การทดสอบ packet สังเคราะห์ไม่ใช่การยืนยันจาก F1 25 จริง และการ build บน macOS ไม่ใช่การทดสอบบน Windows

ฟอนต์ Noto Sans Thai และ Barlow Condensed อยู่ใน `assets/fonts` พร้อม SIL Open Font License; ไอคอนใช้ไฟล์เดียวกับโปรเจ็คเดิม

ไอคอน Windows ฝังใน `.exe` ผ่าน `build.rs` และ `assets/windows.rc` โดยใช้ `assets/icon.ico` หลายขนาด (16–256 px) ส่วนไอคอนหน้าต่างใช้ `assets/icon.png` จากภาพเดียวกัน Cross-build ต้องมี `llvm-rc` บน PATH หรือระบุ resource compiler ผ่าน `RC`

## ปรับแต่งสไตล์วิดเจ็ต

เลือกวิดเจ็ตแล้วไปที่ **Inspector → APPEARANCE**:

- **Wheel LCD**: จอแข่งพื้นเข้ม ขอบบาง มุมเหลี่ยม ฟอนต์ condensed; **Minimal**: พื้นโปร่งใส ซ่อนหัวข้อ/หน่วย; **Card**: พื้นหลังและกรอบมุมโค้ง ระยะด้านในกว้างขึ้น แสดงค่าด้วยฟอนต์ mono สีขาว ซ่อนหัวข้อ/หน่วยแบบ Minimal แต่ละปุ่มใช้กับวิดเจ็ตที่เลือกเท่านั้น
- **กรอบและระยะห่าง**: มุมโค้ง 0–100, ขอบ 0–20, สีขอบ และระยะด้านใน 0–100 (หน่วยพิกเซลบน canvas)
- **หัวข้อและหน่วย**: ซ่อนหัวข้อ ปรับสี/ขนาด และวางหัวข้อด้านบนหรือด้านล่าง หน่วยของ Speed, Fuel, Throttle และ Brake ปรับสี/ขนาดและซ่อนได้ โดยแสดงแยกด้านล่างตัวเลข
- **จัดตำแหน่งตัวเลข / ข้อความ**: ซ้าย–กลาง–ขวา และบน–กลาง–ล่าง; Tyres และแถบ RPM ใช้การจัดองค์ประกอบเฉพาะของตัวเอง ส่วน Lap times ปรับตำแหน่งค่าในแต่ละแถว
- **ไฟ RPM**: เลือกเหลี่ยม มน หรือ LED วงกลม ใช้ช่วงสี RPM และข้อมูลเดิม
- **คัดลอกสไตล์ → เลือกอีกวิดเจ็ต → วางสไตล์**: คัดลอกสี ฟอนต์ กรอบและการจัดข้อความ โดยเก็บขนาดตัวเลข ตำแหน่ง ชนิดข้อมูล ชื่อ และเกณฑ์เตือนของวิดเจ็ตปลายทาง

ทุกการเปลี่ยนสไตล์ Undo/Redo ได้และบันทึกอัตโนมัติ รวมอยู่ใน JSON ที่ส่งออก เลย์เอาต์เก่านำเข้าได้โดยมีค่าเริ่มต้นให้ สไตล์ใหม่รองรับในแอป Rust; แอป Electron เดิมไม่แสดงการตกแต่งใหม่นี้
