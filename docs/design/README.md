# Дизайн экранов Meshwork

Исходники дизайн-канваса (Claude Design): <https://claude.ai/artifact/GCP5m76zxtkmfhZV2pGwiw>.
Каждый `*.dc.html` — один экран (самодостаточный HTML, открывается в браузере только как референс,
`support.js` не лежит в репозитории); `canvas.json` — раскладка артбордов.

| Файл | Экран |
|---|---|
| Main | Вход / регистрация |
| Graphs | Список графов + «Waiting on you» |
| EmptyGraph | Новый граф (пустое состояние) |
| Graph | Граф-вью (ноды, рёбра BLOCKS / PART_OF / AFFECTS, chore) |
| Board | Канбан по статусам |
| Table | Таблица |
| Task | Страница задачи (blockers, verification, лог, детали) |
| NewTask | Диалог создания задачи |
| User | Профиль: назначенные / на ревью задачи |
| Settings | Настройки |

Токены: шрифты IBM Plex Sans / Mono; фон `#F4F6F8`, текст `#12161C`, акцент `#2349C7`;
статусы TODO `#E8EBEF/#3E4753`, IN_PROGRESS `#DCE8FF/#1B3FA8`, REVIEW `#EADFFB/#5A2BA6`,
DONE `#D5F0E3/#0F5C3A`, BLOCKED `#FFE1D3/#A13A0B`.

Экраны Main (вход), User и Settings, «Waiting on you», шаблоны графов, лог активности и
комментарии во фронте не реализованы: в API нет пользователей, авторизации и истории изменений
(см. TODO.md, «Дыры API для нового UI»).
