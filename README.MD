### Monux
---
**Preview**

[![Смотреть превью Monux](https://img.youtube.com/vi/_FHk2E1aSxw/maxresdefault.jpg)](https://youtu.be/_FHk2E1aSxw "Monux Preview")

### CLI quick start

```bash
monux init
monux new "My first note" --tags "rust cli"
monux list
monux find first
monux find rust --content
monux tags add my-first-note "dev todo"
monux rename my-first-note "My renamed note"
monux sync
monux delete my-renamed-note --yes
```

### Ed-like editor (CLI)

Запуск:

```bash
# открыть заметку по slug из индекса
monux edit my-note

# или открыть пустой интерактивный редактор
monux edit
```

Промпты:

- `:` — обычный командный режим
- `>` — режим ввода текста после `a`/`i`/`c`

Базовые команды:

- `p` — печать текущей строки (или диапазона)
- `n` — печать с номерами строк
- `d` — удалить строку/диапазон
- `a` — добавить строки после адреса
- `i` — вставить строки перед адресом
- `c` — заменить строку/диапазон
- `w` — сохранить в текущий файл
- `w path/to/file.md` — сохранить в другой файл
- `e path/to/file.md` — открыть другой файл
- `q` — выйти (если нет несохранённых изменений)
- `Q` — выйти принудительно

Адресация строк:

- `.` — текущая строка
- `$` — последняя строка
- `N` — строка по номеру, например `12p`
- `A,B` — диапазон, например `1,5p`
- `%` — весь файл (`1,$`)

Как вводить текст:

1. Выполнить `a`, `i` или `c`.
2. Вводить строки по одной (приглашение `>`).
3. Ввести отдельную строку `.` для завершения ввода.

Мини-пример сессии:

```text
: 1,$n
: $a
new line
second line
.
: 1,$p
: w
: q
```
