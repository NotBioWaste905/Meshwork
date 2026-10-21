# Meshwork — Architecture (C4)

Срез архитектуры на текущий момент (ветка `falkordb`, бэкенд переезжает на
Rust). Не путать с [MISSION.md](../MISSION.md) — там продуктовое видение,
здесь — как оно устроено технически. Диаграммы будут расходиться с кодом
по мере разработки, актуализировать по факту значимых изменений.

## Level 1 — System Context

Кто вообще взаимодействует с системой.

```mermaid
C4Context
    title Meshwork — System Context

    Person(user, "Пользователь", "Участник команды: ставит задачи, работает по графу, смотрит канбан/таблицу")
    Person(lead, "Руководитель / ревьюер", "Верифицирует результат задач перед закрытием")

    System(meshwork, "Meshwork", "Граф задач: зависимости, лог изменений, канбан/таблица/граф-вью поверх одной модели")

    System_Ext(vcs, "GitHub / GitLab", "Источник PR/коммитов для artifact-based верификации задач (план)")

    Rel(user, meshwork, "Создаёт и ведёт задачи, строит граф зависимостей")
    Rel(lead, meshwork, "Подтверждает / отклоняет результат задачи")
    Rel(meshwork, vcs, "Забирает статус PR/коммитов как артефакт для верификации", "HTTPS API (план)")
```

## Level 2 — Containers

Что из чего физически состоит сейчас.

```mermaid
C4Container
    title Meshwork — Containers (as-is)

    Person(user, "Пользователь")

    System_Boundary(meshwork, "Meshwork") {
        Container(frontend, "Webapp", "SvelteKit + Svelte 5, Tailwind, @xyflow/svelte + dagre", "Граф-вью, форма задачи. Канбан/таблица — план")
        Container(backend, "Backend API", "Rust, axum, falkordb", "REST /v0/...: CRUD графа и задач, connect/disconnect зависимостей")
        ContainerDb(db, "FalkorDB", "Graph DB (Redis protocol :6379, browser UI :3001 на хосте)", "Ноды Graph/Task/User, рёбра PART_OF/DEPENDS_ON/ASSIGNED_TO (BLOCKS/AFFECTS — план)")
    }

    Rel(user, frontend, "HTTPS", "браузер")
    Rel(frontend, backend, "REST /v0/...", "JSON over HTTP :8000")
    Rel(backend, db, "Redis protocol", "falkordb crate (async)")
```

## Level 2 — Containers (целевое, по MISSION.md)

Куда движемся: те же контейнеры, плюс то, что нужно под прозрачность и
верификацию из [MISSION.md](../MISSION.md).

```mermaid
C4Container
    title Meshwork — Containers (target)

    Person(user, "Пользователь")
    Person(lead, "Ревьюер")

    System_Boundary(meshwork, "Meshwork") {
        Container(frontend, "Webapp", "SvelteKit", "Три проекции одной модели: граф / канбан / таблица")
        Container(backend, "Backend API", "Rust, axum, falkordb", "CRUD графа, вычисление frontier (готовых задач), запись в audit log")
        ContainerDb(db, "FalkorDB", "Graph DB", "Ноды задач/проектов, рёбра BLOCKS/PART_OF/AFFECTS, FieldDefinition (план)")
        ContainerDb(auditlog, "Audit Log store", "FalkorDB (события как ноды) или отдельный append-only store — не решено", "Полная история изменений задач и структуры графа")
        Container(verify, "Verification flow", "часть backend", "Artifact-based верификация: PR/коммит/саммари/аппрув привязаны к задаче перед переводом в done")
    }

    System_Ext(vcs, "GitHub / GitLab", "PR, коммиты, CI-статусы")

    Rel(user, frontend, "HTTPS")
    Rel(lead, frontend, "HTTPS, подтверждает верификацию")
    Rel(frontend, backend, "REST/GraphQL")
    Rel(backend, db, "Redis protocol")
    Rel(backend, auditlog, "запись события на каждое изменение")
    Rel(backend, verify, "проверка условия закрытия задачи")
    Rel(verify, vcs, "статус PR/CI как артефакт")
```

## Открытые архитектурные вопросы

- **Audit log**: писать события как ноды/рёбра в той же FalkorDB (просто, но
  разрастает граф и мешает основным обходам) или в отдельный append-only
  store, синхронизированный по `task_id`? Не решено.
- **Кастомные поля**: отложено см. [MISSION.md § Открытые вопросы](../MISSION.md).
  Если решится в пользу произвольных полей — вероятно, `FieldDefinition`
  ноды на уровне проекта (см. обсуждение), без отдельной document DB.
