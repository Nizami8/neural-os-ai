# RISC-V Milk-V Duo 256M Deployment

## 🎯 Quick Start (5 минут)

### 1. Проверяем Milk-V Duo в сети

```bash
# Подключаем к сети (Ethernet через USB или WiFi через USB adapter)
# На компе:
nmap -sn 192.168.1.0/24 | grep -i milkv

# Увидим что-то типо:
# Nmap scan report for 192.168.1.100
# Host is up (0.052s latency).
```

### 2. Собираем для Milk-V

```bash
# На компе (Linux/Mac)
./build-milkv.sh

# Результат:
# ✅ Build successful!
# 📊 Binary Size: 1024 KB
```

### 3. Разворачиваем

```bash
./deploy-milkv.sh 192.168.1.100

# Увидим:
# 🤖 Neural OS v0.9 - Milk-V Duo
#    256M Optimized Edition
#
# 📊 Initializing 2-core scheduler...
# [T1:0|RT] [T2:0|BG] [T1:1|RT] [T2:1|BG] ...
# 📊 Statistics...
```

---

## 🔧 Детальная установка

### Требования

```
Hardware:
✓ Milk-V Duo 256M board ($12)
✓ MicroSD 8GB+ 
✓ USB-C кабель (питание + UART)
✓ Компьютер (Linux/Mac/Windows)

Software:
✓ Rust + riscv64gc-unknown-linux-gnu target
✓ riscv64-unknown-linux-gnu-gcc toolchain
✓ SSH клиент
✓ Наше Neural OS repo
```

### Установка окружения

```bash
# 1. Клонируем репо с Milk-V поддержкой
git clone https://github.com/Nizami8/neural-os-ai.git
cd neural-os-ai

# 2. Запускаем setup скрипт
chmod +x setup-milkv.sh
./setup-milkv.sh

# 3. Активируем окружение
source ~/.milkv-env

# 4. Проверяем toolchain
riscv64-unknown-linux-gnu-gcc --version
```

---

## 📦 Структура проекта для Milk-V

```
neural-os-ai/
├── src/
│   ├── bin/
│   │   └── milkv-userspace.rs        ← Основной бинарь для Duo
│   ├── neural.rs                     ← MLP сеть (7→8→1)
│   ├── adaptive.rs                   ← Scheduler
│   ├── trap.rs                       ← Обработчики (Linux syscalls)
│   └── scheduler.rs                  ← Task управление
├── .cargo/config.toml                ← Cross-compile конфиг
├── build-milkv.sh                    ← Сборка
├── deploy-milkv.sh                   ← Развертывание
├── setup-milkv.sh                    ← Инициализация окружения
└── docs/MILKV-GUIDE.md               ← Полная документация
```

---

## 🚀 Версии для разных сценариев

### Вариант A: Barebone (никакого Linux)

```bash
# Прошиваем только bootloader + ваш OS
# Загрузка быстрее, контроль полный
# Но нужно реализовать все самостоятельно

./build-milkv.sh --barebone
```

### Вариант B: Linux userspace (рекомендуется)

```bash
# Запускаем как Linux приложение
# Удобнее отладить, меньше проблем
# Немного медленнее, но стабильнее

./build-milkv.sh --userspace    # По умолчанию
```

### Вариант C: Linux kernel module

```bash
# Встраиваем в kernel
# Максимальная производительность
# Требует модификации kernel

./build-milkv.sh --kernel-module
```

---

## 📊 Ожидаемая производительность

На Milk-V Duo 256M (2 ядра @ 1.2 GHz):

```
Startup time:        ~1-2 секунды
Neural inference:    ~5-10 μs per prediction
Context switch:      ~1-2 μs
Memory footprint:    ~50-80 MB (остается ~180 MB свободно)
Fairness index:      0.95+
Throughput:          ~500-1000 context switches/sec
```

---

## 🔍 Отладка на Milk-V Duo

### Serial console (UART)

```bash
# На компе:
sudo screen /dev/ttyUSB0 115200

# Увидишь boot messages, output, panics
# Ctrl+A, затем K для выхода

# Альтернативно:
sudo minicom -D /dev/ttyUSB0 -b 115200
```

### SSH доступ

```bash
# Первая загрузка (пароль по умолчанию)
ssh root@192.168.1.100
password: <none>

# Меняем пароль
passwd

# Теперь можно использовать deploy-milkv.sh
```

### Логирование

```bash
# На Milk-V:
/root/neural-os > /tmp/neural-os.log 2>&1 &

# На компе:
scp root@192.168.1.100:/tmp/neural-os.log .
tail -f neural-os.log
```

---

## ⚠️ Частые проблемы

### Problem: "Cannot connect to Milk-V"

```bash
# Решение 1: Проверяем DHCP
# На Milk-V:
ifconfig
# Должен быть IP адрес

# Решение 2: Статический IP
ssh root@192.168.1.100
vi /etc/config/network
# Меняем DHCP на статический IP
/etc/init.d/network restart
```

### Problem: "SSH key rejected"

```bash
ssh-keygen -R 192.168.1.100
ssh root@192.168.1.100
```

### Problem: "Out of memory"

```bash
# Milk-V имеет только 256 MB
# Если память полная, нужно:
# 1. Урезать Linux (выключить bluetooth, etc)
# 2. Уменьшить METRICS_HISTORY_SIZE
# 3. Компилировать с -C opt-level=z

# Проверяем:
free -h
```

---

## 🎓 Дополнительно

### Собрать свой образ Linux для Milk-V

```bash
cd ~/milkv-workspace/duo-buildroot-sdk
./build.sh menuconfig

# Отключаем ненужное:
# Target packages → Networking → deselect WiFi
# Target packages → Multimedia → deselect Video
# Target packages → Development → deselect debuggers

./build.sh
```

### Перекомпилировать kernel

```bash
cd duo-buildroot-sdk
./build.sh linux-menuconfig

# Отключаем модули для экономии памяти
# Save & Exit

./build.sh
```

---

## 📞 Поддержка

- **GitHub Issues:** https://github.com/Nizami8/neural-os-ai/issues
- **Milk-V Community:** https://github.com/milkv-duo/duo-buildroot-sdk
- **RISC-V Forum:** https://github.com/riscv-admin/riscv-community

---

**Готово! Теперь можешь запустить Neural OS на реальном hardware!** 🚀
