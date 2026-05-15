# Тестирование связки «сервер на Fedora 44 ↔ клиент в ВМ (VAIO P)»

Пошаговая инструкция для проверки запуска `ph0sphor-server` на
рабочей машине под управлением Fedora 44 и `ph0sphor-client`
внутри VirtualBox-ВМ, эмулирующей Sony VAIO P (antiX 32-bit или
аналог). Цель — убедиться, что бинари стартуют, проходят парность
и клиент отображает реальные данные с сервера.

Предполагается, что сервер и клиент уже установлены по
[`installation.md`](installation.md) и
[`vaio-p-client-vm-testing.md`](vaio-p-client-vm-testing.md).

## 0. Предполётная проверка

На хосте (Fedora 44):

```sh
ph0sphor-server --version
ph0sphorctl --version
```

В ВМ (клиент):

```sh
ph0sphor-client --version
```

Все три команды должны вернуть `0.1.0` (или текущий релиз).
Если бинарей нет в `PATH` — вернитесь к шагам установки.

## 1. Изолированный smoke-тест через `--demo`

Цель: убедиться, что бинари в принципе работают на каждой стороне,
без сети и парности.

### 1.1 Сервер на хосте

В отдельном терминале на Fedora:

```sh
ph0sphor-server --demo
```

Ожидаемый вывод (примерно):

```text
INFO ph0sphor_server: starting in demo mode
INFO ph0sphor_server: ws bind 0.0.0.0:7077
INFO ph0sphor_server: control bind 127.0.0.1:7078
```

Оставьте окно открытым.

### 1.2 Клиент в ВМ

В терминале ВМ:

```sh
ph0sphor-client --demo
```

Должен открыться TUI с синтетическими CPU/RAM/диск/сеть.
Проверьте навигацию:

- `Tab` / `Shift+Tab` — переключение шести экранов.
- `1`–`6` — прямой переход на экран.
- `C` — циклическая смена темы (`phosphor-green` → `amber-crt` → …).
- `Q` или `Ctrl+C` — выход без артефактов в терминале.

Если хоть одна сторона в `--demo` не запускается — дальше идти
бессмысленно, читайте лог и раздел «Troubleshooting» в
[`installation.md`](installation.md).

## 2. Подготовка сети между хостом и ВМ

Сервер слушает `0.0.0.0:7077` (см. `examples/server.toml`).
Клиент должен видеть этот порт по сети из ВМ.

### 2.1 Открыть порт в firewalld (Fedora 44)

Fedora по умолчанию использует firewalld, и порт 7077 закрыт.
Откройте его в активной зоне (обычно `FedoraWorkstation`):

```sh
# временно — на текущую сессию
sudo firewall-cmd --add-port=7077/tcp

# постоянно
sudo firewall-cmd --permanent --add-port=7077/tcp
sudo firewall-cmd --reload
```

Контрольный порт `7078` оставляем закрытым — он биндится только
на `127.0.0.1` и используется `ph0sphorctl` локально.

### 2.2 Сетевой режим VirtualBox

Выберите один из вариантов настройки адаптера ВМ:

- **NAT (по умолчанию)**. Хост виден из гостя как `10.0.2.2`.
  Никаких пробросов настраивать не нужно — соединение «гость → хост»
  по NAT работает из коробки.
- **Bridged**. ВМ получает IP в той же подсети, что и хост.
  Используйте реальный IP воркстейшна: `ip -4 addr show` на хосте.
- **Host-only**. Создайте host-only сеть в VirtualBox, ВМ получит
  адрес из неё, хост будет доступен по выделенному IP (обычно
  `192.168.56.1`).

### 2.3 Проверить достижимость порта из ВМ

В ВМ:

```sh
# для NAT
nc -zv 10.0.2.2 7077

# для Bridged / Host-only — подставить реальный IP хоста
nc -zv 192.168.1.42 7077
```

Ответ `succeeded` или `open` означает, что путь от клиента до
сервера свободен. `Connection refused` — сервер не запущен или
слушает не тот интерфейс. `No route to host` / `timed out` —
firewall либо неправильный сетевой режим VBox.

## 3. Боевой запуск сервера

Остановите `--demo`-сервер из шага 1 (`Ctrl+C`).

### 3.1 Конфиг

Поправьте `/etc/ph0sphor/server.toml` (или скопируйте
`examples/server.toml` под другое имя и используйте `--config`):

```toml
[server]
bind = "0.0.0.0:7077"
control_bind = "127.0.0.1:7078"
name = "main-pc"

[security]
pairing_enabled = true
require_token = true
token_store = "/var/lib/ph0sphor/tokens.json"
```

Создайте директорию для токенов:

```sh
sudo mkdir -p /var/lib/ph0sphor
sudo chmod 0700 /var/lib/ph0sphor
```

### 3.2 Запуск

Под systemd (рекомендованный путь, юнит ставится из
`packaging/linux/ph0sphor-server.service`):

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now ph0sphor-server.service
sudo systemctl status ph0sphor-server.service
journalctl -u ph0sphor-server.service -f
```

Или вручную, в отдельном терминале:

```sh
RUST_LOG=info ph0sphor-server --config /etc/ph0sphor/server.toml
```

В логе должны появиться строки про bind на `0.0.0.0:7077` и
старт коллекторов CPU / memory / disk / network.

## 4. Боевой запуск клиента и парность

В ВМ убедитесь, что в `~/.config/ph0sphor/client.toml`:

```toml
[client]
# для NAT
server = "ws://10.0.2.2:7077/ws"
# для Bridged / Host-only — подставьте IP хоста
# server = "ws://192.168.1.42:7077/ws"
client_name = "vaio-p-vm"
token = ""
token_file = "~/.config/ph0sphor/token"
theme = "phosphor-green"

[ui]
ascii_fallback = true
compact_mode = true
```

Запустите клиент:

```sh
ph0sphor-client --config ~/.config/ph0sphor/client.toml
```

В шапке появится баннер `PAIRING` с кодом вида `ABCD-1234`:

```text
+- PAIRING ----------------------------+
|  PAIRING REQUIRED                    |
|  CODE: ABCD-1234                     |
|  On the server host, confirm with:   |
|    ph0sphorctl pair confirm ABCD-1234|
|  (awaiting operator...)              |
+--------------------------------------+
```

На хосте (Fedora) подтвердите код:

```sh
ph0sphorctl pair confirm ABCD-1234
# pairing confirmed
```

`ph0sphorctl` бьёт в loopback `127.0.0.1:7078`, выдаёт 192-битный
токен, сохраняет его в `/var/lib/ph0sphor/tokens.json` и пушит
ожидающему клиенту по той же WebSocket. Клиент сохранит токен в
`~/.config/ph0sphor/token` (mode 0600) и переподключится.

Шапка должна смениться на:

```text
LINK: ONLINE   HOST: main-pc   UP: hh:mm:ss   BAT: -- DSC   NET: ...
```

> В ВМ батарея обычно отсутствует — `BAT:` будет пустым или
> `--`, это ожидаемо.

Перезапустите клиент — парность больше запрашиваться не должна,
сразу `LINK: ONLINE`.

## 5. Проверка вывода данных

С `LINK: ONLINE` сервер шлёт реальные метрики хоста (Fedora 44).
Пройдитесь по экранам и сверьте.

| Кл. | Экран     | Что должно быть видно                                             |
| --- | --------- | ----------------------------------------------------------------- |
| `1` | HOME      | Сводка: CPU%, RAM, диск, сеть, аптайм, имя хоста                  |
| `2` | CPU       | Загрузка по ядрам, бар-чарты, среднее за 1/5/15 мин                |
| `3` | MEM       | Used/Free/Cache, swap, top-3 потребителя                           |
| `4` | NET       | Активные интерфейсы, RX/TX-скорости, IP                            |
| `5` | DISK      | Использование разделов, free %, IO                                 |
| `6` | EVENTS    | Поток событий: коннект, парность, ошибки коллекторов               |

Тонкие проверки на хосте:

```sh
# нагрузить CPU и убедиться, что клиент реагирует
yes > /dev/null & sleep 10; kill %1

# забить трафик по wlan/eth — проверить экран NET
curl -o /dev/null https://speed.cloudflare.com/__down?bytes=104857600
```

Бары и числа на клиенте должны меняться в пределах 1–2 секунд
(см. `[performance].main_tick_ms` и `[client].render_fps`).

## 6. Проверка отказоустойчивости

Без этих сценариев тест считается неполным:

- [ ] Выключить сетевой адаптер ВМ в VirtualBox → клиент уходит
      в `LINK: OFFLINE`, не падает, события про разрыв пишутся в
      экран EVENTS.
- [ ] Включить адаптер обратно → автоматический реконнект,
      `LINK: ONLINE` без повторной парности.
- [ ] `sudo systemctl restart ph0sphor-server` на хосте → клиент
      переподключается, токен прежний, парность не требуется.
- [ ] Удалить `~/.config/ph0sphor/token` в ВМ и перезапустить
      клиент → снова появляется `PAIRING`-баннер с новым кодом.
- [ ] `Q` и `Ctrl+C` в клиенте корректно возвращают терминал в
      нормальный режим (никаких «битых» цветов и курсора).

## 7. Откат к чистому состоянию

Если хотите прогнать тест парности заново:

На хосте:

```sh
sudo systemctl stop ph0sphor-server
sudo rm -f /var/lib/ph0sphor/tokens.json
sudo systemctl start ph0sphor-server
```

В ВМ:

```sh
rm -f ~/.config/ph0sphor/token
```

После этого следующий запуск клиента снова попадёт на шаг 4.

## Связанные документы

- [`installation.md`](installation.md) — полный путь установки.
- [`vaio-p-client-vm-testing.md`](vaio-p-client-vm-testing.md) —
  специфика antiX/VirtualBox со стороны клиента.
- [`vaio-p-client.md`](vaio-p-client.md) — рецепт для реального
  VAIO P (шрифты, autologin, console mode).
- [`security-model.md`](security-model.md) — детали парности,
  токенов и контрольного канала.
- [`configuration.md`](configuration.md) — полный справочник по
  TOML-ключам сервера и клиента.
