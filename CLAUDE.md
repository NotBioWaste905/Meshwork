# Meshwork

Таск-трекер, где задачи — ноды графа, а зависимости видны по умолчанию.
Продуктовое видение — [MISSION.md](MISSION.md), архитектура (C4) —
[docs/architecture.md](docs/architecture.md), бэклог — [TODO.md](TODO.md).
Перед изменением модели данных (типы нод/рёбер, статусы) сверяйся с
MISSION.md § «Идея графа»: там зафиксированы `BLOCKS` / `PART_OF` /
`AFFECTS`, `CHORE`-ноды и отказ от ИЛИ-зависимостей.

## Структура

```
compose.yaml                 # backend + frontend + falkordb
meshwork/backend_rs/         # Активный бэкенд: Rust, axum 0.6, falkordb crate
  src/main.rs                #   бутстрап: env, подключение к FalkorDB, запуск сервера
  src/lib.rs                 #   роутер (app()), CORS, HTTP-хендлеры
  src/db.rs                  #   все Cypher-запросы
  src/frontier.rs            #   ready_tasks: чистая логика frontier, юнит-тесты
  src/schemas.rs             #   контракт API: Task, NewTask, TaskUpdate, Status
  src/error.rs               #   AppError -> HTTP-ответ
  tests/api.rs               #   интеграционные тесты против живой FalkorDB
meshwork/frontend/webapp/    # SvelteKit 2 + Svelte 5 (runes), Tailwind 4, @xyflow/svelte + dagre
  src/lib/api.ts             #   единственный HTTP-клиент к бэкенду, зеркало schemas.rs
  src/lib/graphStore.svelte.ts #  задачи открытого графа, общие для всех экранов графа
  src/routes/graphs/         #   список графов (известных этому браузеру, localStorage) и new/: создание
  src/routes/graph/          #   +page.svelte: редирект на последний/новый граф
                             #   [id]/+page.svelte: граф-вью; TaskNode.svelte: нода и модалка редактирования
                             #   [id]/board, [id]/table, [id]/task/[taskId]: канбан, таблица, страница задачи
                             #   Фейковых данных нет: чего нет в API, того нет в UI (TODO.md, «Дыры API»)
meshwork/backend/            # LEGACY: Python/FastAPI, in-memory граф + networkx. Не запускается в compose.
                             #   Только как референс логики при переносе в Rust.
```

## Команды

```bash
# Всё вместе
docker compose up -d --build        # frontend :3000, backend :8000, FalkorDB :6379, FalkorDB UI :3001

# Бэкенд локально (нужна FalkorDB: docker compose up -d falkordb)
cd meshwork/backend_rs
FALKORDB_URL=falkor://localhost:6379 cargo run
cargo clippy --all-targets && cargo fmt
cargo test -- --ignored             # интеграционные тесты, нужна FalkorDB на localhost:6379

# Фронтенд
cd meshwork/frontend/webapp
npm ci && npm run dev               # :5173; бэкенд разрешает CORS с :3000 и :5173
npm run check      # svelte-check / TS
npm run lint       # prettier --check + eslint
npm run format
```

Переменные бэкенда: `FALKORDB_URL` (default `falkor://falkordb:6379`),
`FALKORDB_GRAPH` (default `meshwork`), `CORS_ALLOW_ORIGINS` (через запятую
или `*`, default `http://localhost:3000,http://localhost:5173`). Все
пользовательские «графы» живут в одном FalkorDB-графе и различаются нодой
`(:Graph {id})`; тесты создают отдельный FalkorDB-граф `test_<uuid>` и удаляют его.

Фронт: `VITE_API_URL` (build-time, default `http://localhost:8000`), см. `.env.example`.

CI: [.github/workflows/ci.yml](.github/workflows/ci.yml) — на push и PR
в `main`/`dev`. Бэкенд: `cargo fmt --check`, `clippy -D warnings`,
интеграционные тесты против FalkorDB в service-контейнере. Фронт:
`npm run check`, `npm run lint`, `npm run build`. Версия FalkorDB
запинена в двух местах — `compose.yaml` и `ci.yml`, держи их одинаковыми.

## Модель в БД (как сейчас в коде)

```
(:Task {id, name, description, status, tags: [string]})-[:PART_OF]->(:Graph {id})
(:Task)-[:DEPENDS_ON]->(:Task)      # t зависит от dep
(:Task)-[:ASSIGNED_TO]->(:User {id})
```

`status` хранится той же строкой, что ходит по API: `TODO`, `IN_PROGRESS`,
`DONE`, `REVIEW`, `BLOCKED` (`Status::as_str`). Старое `InProgress`
читается через serde alias.

API (`/v0/...`): `POST create_graph`; `GET :graph/all_tasks`;
`GET :graph/ready_tasks[?unassigned_only=true]` (frontier: `TODO`, все
зависимости `DONE`);
`POST :graph/task/add/` (тело `NewTask`, обязательно только `name`, ответ —
созданный `Task`, 201); `GET|PUT|DELETE :graph/task/:id` (PUT — частичный
апдейт, списки заменяются целиком, лишние поля игнорируются);
`POST :graph/connect_nodes/` и `disconnect_nodes/` с
`{node_dependency, node_dependee}` — dependency блокирует dependee, на
канвасе ребро идёт dependency → dependee. Ошибки — `{message}` с 400/404/500;
409 — если новая зависимость замкнула бы цикл `DEPENDS_ON` (граф
зависимостей всегда DAG).

`DEPENDS_ON` — это и есть блокирующее ребро, которое MISSION.md называет
`BLOCKS`: `(a)-[:DEPENDS_ON]->(b)` означает «b блокирует a». Имя в коде
остаётся `DEPENDS_ON`, переименовывать не нужно.

## Нюансы и грабли

- **FalkorDB, не Neo4j.** Переезд
  Neo4j → FalkorDB сделан. Cypher почти тот же, но драйвер — `falkordb`
  (Redis-протокол), транзакций на несколько запросов нет: каждый
  `query()` атомарен сам по себе, хендлер целиком — нет.
- Параметры запросов только через `.with_param(s)`, никогда не
  форматировать значения в строку Cypher.
- Хендлеры с несколькими запросами сначала валидируют (граф/задачи
  существуют), потом пишут — чтобы отклонённый запрос не оставлял
  полузаписанного состояния. Держи этот порядок.
- `#![recursion_limit = "256"]` в `lib.rs` нужен: на свежем rustc
  async-футуры falkordb не влезают в дефолтный лимит.
- Если `npm ci` получает 404 — глобальный npm-конфиг может смотреть в
  корпоративный реестр; `package-lock.json` ссылается на
  `registry.npmjs.org`, ставь с `--registry=https://registry.npmjs.org`.
- Фронт: Svelte 5 runes (`$state`, `$props`), не Svelte 4-синтаксис.
  Отступы — табы (prettier).

## Конвенции

- Ветки: `main` — стабильная, `dev` — интеграционная. Каждая задача из
  TODO.md — короткая ветка от `dev` (`feat/…`, `fix/…`, `ci/…`, `docs/…`),
  вливается PR-ом в `dev` после зелёного CI. `dev` → `main` — когда набралось
  законченное (например, фаза бэклога). Отмечай сделанное в TODO.md в той же ветке.
- Сообщения коммитов — короткие, на английском, в повелительном
  наклонении (`Add …`, `Fix …`).
- Документация проекта (MISSION, architecture, TODO) — на русском.
- При значимых изменениях архитектуры обновляй docs/architecture.md.
