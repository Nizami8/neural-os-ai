# Milk-V Duo 256M - Neural OS Implementation Guide

## 🎯 Setup для Milk-V Duo 256M

### Характеристики Milk-V Duo 256M

```
CPU:        Dual-core SG2042 @ 1.2 GHz (RISC-V)
RAM:        256 MB DRAM
Storage:    microSD (bootable)
GPIO:       40-pin header (I2C, SPI, UART)
USB:        1x USB Type-A (host), 1x USB Type-C (device/power)
Ethernet:   None (add via USB adapter)
Power:      5V @ 1A via USB-C
Size:       45mm × 50mm (tiny!)
```

### Ограничения на 256MB

```
✅ Хватает на:
   - Ваш Neural OS kernel
   - MLP нейросеть (7→8→1)
   - 2 ядра multi-core scheduler
   - Metrics collection
   - Persistent storage

⚠️ Ограничивает:
   - Не сможешь запустить полный Linux (нужно урезать)
   - Limited history для metrics (~100 samples)
   - Нет места для больших логов
   - Батарейное питание только с USB power bank
```

---

## 🔧 Шаг 1: Загрузка и прошивка

### 1.1 Скачиваем SDK

```bash
cd ~/milkv-workspace/duo-buildroot-sdk

# Конфигурируем для 256M версии
./build.sh menuconfig

# В меню выбираем:
# Board Selection → SG2042
# Memory Layout → 256 MB
# Save & Exit
```

### 1.2 Собираем образ

```bash
./build.sh

# Ждем 20-30 минут (компилирует Linux kernel)
# Результат в: out/milkv-duo-256m.img
```

### 1.3 Прошиваем SD карту

```bash
# Linux/Mac
sudo dd if=out/milkv-duo-256m.img of=/dev/sdX bs=4M
sudo sync

# Windows (используй Etcher или Rufus)
# 1. Скачай Balena Etcher: https://www.balena.io/etcher/
# 2. Select image: out/milkv-duo-256m.img
# 3. Select drive: твоя microSD
# 4. Flash
```

### 1.4 Первая загрузка

```bash
# Вставь microSD в Duo
# Подключи USB-C кабель (питание + консоль)
# На компе:
sudo screen /dev/ttyUSB0 115200

# Увидишь:
# [MILKV] Starting bootloader...
# [MILKV] Starting Linux kernel...
# Welcome to Milk-V Duo!
# login: root
# password: (просто Enter)

# Сразу после первого входа задайте пароль root командой passwd
# или установите SSH-ключ. До этого не используйте deploy-milkv.sh:
# он загружает и запускает бинарник от имени root.
```
