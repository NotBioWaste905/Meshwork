# Эксплуатация: данные FalkorDB и токены

FalkorDB — модуль Redis: весь граф живёт в памяти, на диск его пишет Redis.
Данные лежат в docker-томе `meshwork_falkordb_data`
(`/var/lib/falkordb/data` в контейнере).

## Как сохраняются данные

- **AOF (журнал)** включён в `compose.yaml`: `--appendonly yes --appendfsync everysec`.
  Каждая запись попадает на диск не позже чем через ~1 с, поэтому при падении
  контейнера или машины теряется максимум последняя секунда.
- **RDB (снимок)** `dump.rdb` Redis делает по своим правилам
  (`save 3600 1 300 100 60 10000`) и при штатной остановке. Из него делаются бэкапы.

> ⚠️ `docker compose down -v` удаляет том вместе со всеми данными.
> Для остановки — `docker compose down` или `docker compose stop`.

## Бэкап

```bash
scripts/backup-falkordb.sh                 # в ~/meshwork-backups
scripts/backup-falkordb.sh /path/to/dir    # или в свою папку
```

Скрипт делает `BGSAVE` в работающем контейнере, ждёт окончания и копирует снимок
в `falkordb-YYYYmmdd-HHMMSS.rdb`. Хранит `MESHWORK_BACKUP_KEEP` (по умолчанию 14)
последних файлов, старые удаляет. Папку по умолчанию можно задать через
`MESHWORK_BACKUP_DIR`.

Раз в день на маке — через launchd или cron, например `crontab -e`:

```
0 3 * * * cd /path/to/Meshwork && scripts/backup-falkordb.sh >> ~/meshwork-backups/backup.log 2>&1
```

Если бэкапы лежат на том же диске, что и Docker, это защита от ошибок и порчи
данных, но не от смерти диска — папку стоит синхронизировать куда-то ещё.

## Восстановление

```bash
docker compose stop falkordb backend
scripts/restore-falkordb.sh ~/meshwork-backups/falkordb-YYYYmmdd-HHMMSS.rdb
docker compose up -d falkordb backend
```

Скрипт спросит подтверждение (нужно ввести `restore`) и **заменит все текущие
данные** в томе.

## Грабли: включение AOF на существующих данных

Если в томе есть только `dump.rdb` и запустить Redis сразу с `--appendonly yes`,
он создаст пустой журнал и **поднимет пустую базу** (журнал при загрузке
главнее снимка). Проверено на копии данных.

Правильный порядок — тот же, что в `restore-falkordb.sh`: стартовать без AOF
(загрузится `dump.rdb`), выполнить `redis-cli CONFIG SET appendonly yes`,
дождаться окончания перезаписи журнала (`aof_rewrite_in_progress:0` в
`INFO persistence`), и только потом запускать с AOF в конфиге. Самый простой
способ для существующего тома — сделать бэкап и сразу восстановить его тем же
скриптом.

## Токены актёров

Каждый запрос к API (`/v0/…`) и MCP идёт от актёра: заголовок `Authorization: Bearer mw_…`.
Без токена бэкенд отвечает 401, неизвестный, отозванный или просроченный токен — тоже 401;
открыты только `/` и `/health`. Токены выдаёт CLI `meshwork-token` в контейнере бэкенда (первый
токен по HTTP получить нельзя, экрана логина нет); дальше актёр может выпускать и отзывать свои
токены сам — на странице «You» (`/me`) в UI или через `/v0/me/tokens`. В базе хранится лишь
SHA-256 токена: потерянный токен не восстановить — выпустите новый и отзовите старый.

```bash
# выпустить токен актёру (id или имя); с --kind актёр создаётся, если его ещё нет
docker compose exec -T backend meshwork-token create ann --kind human --label "ann, браузер"
docker compose exec -T backend meshwork-token create claude --label "claude, ноутбук"
# токен со сроком жизни в днях (1–3650; без --expires-in — до отзыва)
docker compose exec -T backend meshwork-token create claude --expires-in 90
# список токенов (id, актёр, создан, истекает или «-», метка) — сами токены не показываются
docker compose exec -T backend meshwork-token list [claude]
# отозвать по id из списка
docker compose exec -T backend meshwork-token revoke <token-id>
```

`create` печатает в stdout только токен (`mw_…`), а в stderr — его id. Id нужен лишь для
`revoke`; это не токен, с ним API и MCP ответят 401 «Not a Meshwork token». Слияние актёров
переносит их токены, удаление актёра удаляет и токены.

**Вход в веб-интерфейс.** UI тоже ходит в API с токеном: выпустите себе токен командой выше и
вставьте его на странице «You» (`/me`), он хранится в localStorage браузера. Без токена любой
экран перенаправляет на `/me`.

**Режим совместимости.** `MESHWORK_ALLOW_ANONYMOUS=true` у бэкенда (в `compose.yaml` стоит
`false`, по умолчанию тоже выключено) снова обслуживает запросы без токена — анонимно, как до
обязательных токенов: задачи без автора и reporter-а, комментарии и `claim` по-прежнему требуют
токен. Только для локальной разработки и старых клиентов; при старте бэкенд пишет в лог
предупреждение.

**Чего здесь нет.** Ограничения неудачных попыток нет: токен — 244 случайных бита, перебрать его
нельзя, а лимит по адресу клиента уместнее в обратном прокси перед бэкендом, вместе с TLS (сейчас
API и MCP слушают HTTP, токен идёт открытым текстом — не выставляйте порты наружу без TLS).
Сравнение токена за постоянное время не нужно: бэкенд ищет токен по SHA-256 индексом в FalkorDB
и не сравнивает секреты в своём коде.

### Подключение Claude Code к MCP

`.mcp.json` берёт токен из переменной `MESHWORK_MCP_TOKEN`. Проще всего выпустить его скриптом:

```bash
scripts/mcp-token.sh                # токен для claude → .mcp-token (в .gitignore, права 600)
scripts/mcp-token.sh --launchctl    # то же + launchctl setenv для десктопного приложения (macOS)
scripts/mcp-token.sh ann ~/ann.tok  # другой актёр и файл
```

Скрипт проверяет новый токен через `GET /v0/me` и подсказывает, как задать переменную.
Две ловушки:

- **Переменная читается только при запуске.** После замены токена перезапустите Claude Code;
  до этого MCP-сервер так и останется отключённым.
- **Десктопное приложение не видит переменных shell** (`~/.config/fish`, `~/.zshrc`): запущенное
  из Dock, оно берёт окружение у launchd. Задайте `launchctl setenv MESHWORK_MCP_TOKEN "$(cat
  .mcp-token)"` (или `--launchctl` у скрипта), затем закройте приложение целиком (Cmd+Q) и
  откройте снова. `launchctl setenv` живёт до перезагрузки или выхода из системы. Проверить, что
  видит launchd: `launchctl getenv MESHWORK_MCP_TOKEN`.

Проверить токен вручную: `curl -H "Authorization: Bearer $(cat .mcp-token)" localhost:8000/v0/me`
— 200 и актёр в ответе. Старые токены актёра остаются рабочими: `meshwork-token list claude`,
затем `revoke` ненужных.
