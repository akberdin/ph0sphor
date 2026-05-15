# Тестирование клиента на виртуальной машине (antiX Full 32-bit)

Краткая инструкция для прогонки `ph0sphor-client` в VirtualBox-ВМ
с antiX Full 32-bit в качестве заменителя реального Sony VAIO P.
Предполагается, что ВМ уже создана, разрешение экрана настроено
и применены минимальные системные характеристики.

antiX 32-bit совместим с i686-сборкой клиента, поэтому процедура
та же, что и для реального VAIO P. Ниже — минимальный путь
«бинарь → запуск → парность».

## 1. Получить бинарь

Текущий релиз — **v0.1.0**. Для VAIO P / antiX-32 публикуется
статический musl-бинарь `ph0sphor-v0.1.0-linux-i686.tar.gz`,
дополнительные библиотеки в гостевой ОС не нужны.

### Вариант A. Готовый релиз (рекомендуется)

Внутри ВМ:

```sh
REL=v0.1.0
wget https://github.com/akberdin/ph0sphor/releases/download/$REL/ph0sphor-$REL-linux-i686.tar.gz
wget https://github.com/akberdin/ph0sphor/releases/download/$REL/ph0sphor-$REL-linux-i686.tar.gz.sha256
sha256sum -c ph0sphor-$REL-linux-i686.tar.gz.sha256
tar xzf ph0sphor-$REL-linux-i686.tar.gz
cd ph0sphor-$REL-linux-i686
```

В директории появятся `ph0sphor-client` и `examples/client.toml` —
их использует шаг 2.

### Вариант B. Сборка из исходников

Нужна, только если хотите гонять не релизную сборку (например,
проверить незарелизенные изменения с ветки).

На хосте с установленным Rust:

```sh
git clone https://github.com/akberdin/ph0sphor.git
cd ph0sphor
rustup target add i686-unknown-linux-musl
cargo build --release -p ph0sphor-client --target i686-unknown-linux-musl
```

Бинарь будет в `target/i686-unknown-linux-musl/release/ph0sphor-client`
— перенесите его в ВМ через общую папку VirtualBox, `scp` или USB.
Не забудьте также скопировать `examples/client.toml`.

## 2. Поставить бинарь и конфиг

Внутри ВМ, из директории с перенесённым бинарём и `examples/`:

```sh
sudo install -Dm755 ph0sphor-client /usr/local/bin/ph0sphor-client
install -Dm640 examples/client.toml ~/.config/ph0sphor/client.toml
```

В `~/.config/ph0sphor/client.toml` поправить:

- `client.server` — адрес сервера на хост-машине. Из VirtualBox NAT
  хост обычно доступен как `10.0.2.2:7077`; либо используйте режим
  Bridged и реальный IP воркстейшна.
- `client.theme = "phosphor-green"` (или `"amber-crt"`).
- Если шрифт antiX не тянет Unicode-рамки —
  `ui.ascii_fallback = true`.

## 3. Проверить без сети — `--demo`

```sh
ph0sphor-client --demo
```

Это покажет синтетические метрики и подтвердит, что бинарь стартует
под antiX. `Tab` — листать экраны, `C` — темы, `Q` — выход.

## 4. Подключение к серверу + парность

На воркстейшне (хост) запустить сервер с включённой парностью
(`security.pairing_enabled = true`, валидный
`security.token_store`).

На клиенте:

```sh
ph0sphor-client --config ~/.config/ph0sphor/client.toml
```

В шапке появится `PAIRING` с кодом вида `ABCD-1234`. На хосте:

```sh
ph0sphorctl pair confirm ABCD-1234
```

Клиент сам сохранит токен в `~/.config/ph0sphor/token` (0600) и
переключится в `LINK: ONLINE`.

## 5. Тонкости под antiX / VirtualBox

- **systemd-юнит из `packaging/linux/` НЕ применим** — antiX по
  умолчанию на sysVinit/runit. Для теста просто запускайте бинарь
  руками из терминала. Если нужен автостарт на tty1 — секция
  «Bare TTY autologin» в [`vaio-p-client.md`](vaio-p-client.md)
  (agetty + `.profile`).
- **Сеть до сервера**: проверьте доступность из ВМ —
  `nc -zv 10.0.2.2 7077` (NAT) или `nc -zv <ip_хоста> 7077`
  (Bridged). Для NAT не забудьте проброс порта или используйте
  Bridged-адаптер.
- **Шрифт консоли**: `sudo setfont ter-v18n` для CRT-вида; в окне
  терминала antiX подойдёт любой моноширинный.
- **Размер окна**: VAIO P — 1600×768; в VBox ставьте близкое
  разрешение, чтобы воспроизвести «компактный» режим. При меньшем
  размере включите `ui.compact_mode = true`.

## 6. Что проверить в тесте

- [ ] `--demo` стартует и проходит все шесть экранов через `Tab`.
- [ ] Тема переключается по `C` (`phosphor-green` ↔ `amber-crt`).
- [ ] `Q` и `Ctrl+C` корректно выходят, терминал не остаётся в raw-режиме.
- [ ] При запуске без токена показывается PAIRING-баннер с кодом.
- [ ] После `ph0sphorctl pair confirm` клиент уходит в `LINK: ONLINE`.
- [ ] Токен сохранён в `~/.config/ph0sphor/token` с правами 0600.
- [ ] При повторном запуске парность не запрашивается.
- [ ] Заголовок показывает локальные `BAT:` и `NET:` (в ВМ батарея
      может отсутствовать — это ожидаемо).
- [ ] При обрыве сети (выключить адаптер в VBox) клиент остаётся
      отзывчивым и показывает `LINK: OFFLINE`, не падая.

Полные подробности: [`installation.md`](installation.md) и
[`vaio-p-client.md`](vaio-p-client.md).
