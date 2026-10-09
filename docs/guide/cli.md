# The CLI: `cargo rok-ui`

Install it with `cargo install rok-ui-cli`; it runs as `cargo rok-ui`.

| Command | Does |
|---|---|
| `new <name> [--template minimal\|full\|db\|bloc [--http]]` | Create an app. `bloc` is the BLoC architecture with a generated notes feature; `--http` adds an API client, sign-in and a session guard. |
| `generate` (`g`) | Write a feature, or part of one, and wire it in (below). |
| `add <component>...` | Copy components' source into the app to change them. |
| `routes` | Write the route tree for `src/routes` to a checked-in file. |

## `cargo rok-ui generate`

```text
cargo rok-ui g feature <name>       models, provider, repository, bloc, page, route, tests
cargo rok-ui g bloc <Name>          a bloc with its tests (the data layer when missing)
cargo rok-ui g cubit <Name>         a cubit with its tests (the data layer when missing)
cargo rok-ui g repository <Name>    a repository (its provider and models when missing)
cargo rok-ui g provider <Name>      a provider (its models when missing)
cargo rok-ui g view <feature>       a page and its route
cargo rok-ui g api <feature>        an HTTP provider, its DTOs and its repository
cargo rok-ui g schema               the JSON Schema of the -j payload
```

Everything can come from flags, from JSON (`-j '{..}'`, `-j @spec.json`, or `-j -` for
standard input), or both. **Flags win over JSON, and JSON wins over defaults.**

### Flags

| Flag | Means |
|---|---|
| `-j`, `--json <JSON\|@file\|->` | The whole spec |
| `--feature <name>` | The feature, for `bloc`, `cubit`, `repository`, `provider` |
| `--event <Name[:field:Type,...]>` | An event (repeatable; replaces the JSON's events) |
| `--kind <bloc\|cubit>` | |
| `--concurrency <sequential\|droppable\|restartable\|concurrent>` | The bloc's default |
| `--state-style <struct\|enum>` | A struct with a status (default), or an enum of statuses |
| `--status <a,b,c>` | Statuses (default `initial,loading,success,failure`) |
| `--repository <Name>` | |
| `--provider <Name[:memory\|db\|http\|file]>` | |
| `--api-base <path>` | The HTTP provider's path prefix |
| `--endpoint <method:VERB:/path:Response>` | An HTTP endpoint (repeatable); `/notes/{id:NoteId}` types a path parameter |
| `--dto <Name[:field:Type,...]>` | A wire type (repeatable) |
| `--skip-expire` | The `--endpoint`s do not expire the session on a 401 |
| `--server-errors` | The form maps API errors onto its fields |
| `--view` / `--no-view` | Write the page (default: with `feature` only) |
| `--no-tests` | Skip `tests/features/<feature>/` |
| `--path <dir>` | The project (default: the current directory) |
| `--dry-run` | Print what would change; write nothing |
| `--force` | Overwrite generated files that changed |
| `--no-wire` | Leave barrel files, `src/app.rs` and `src/routes/__root.rs` alone |

Exit codes: `0` done, `1` invalid input (the message names the flag or the JSON path), `2`
conflicts (nothing was written).

### What it writes

For `g feature notes --event NotesRequested --event NoteAdded:title:String,body:String`:

```text
src/data/models/note.rs                       Note { id: NoteId, title, body }, NoteId
src/data/error.rs                             DataError (non-HTTP providers)
src/data/providers/notes_memory_provider.rs   in memory: all, find, insert, update, remove
src/data/repositories/notes_repository.rs     trait NotesRepository { list, add } + impl
src/features/notes/bloc/notes_event.rs
src/features/notes/bloc/notes_state.rs        NotesState { status, notes, error }
src/features/notes/bloc/notes_bloc.rs         loading -> call -> success or failure
src/features/notes/view/notes_page.rs         loads once, list, a form for NoteAdded
src/routes/notes.rs                           thin route
tests/features/notes/notes_bloc_test.rs       one test per event, fake repository
```

plus the barrel files (`pub mod` lines kept sorted), a repository and a bloc registered in
`src/app.rs` at `// rok-ui:repositories` and `// rok-ui:blocs`, and a link in
`src/routes/__root.rs` at `// rok-ui:nav`. Without a marker, it prints the line to add.

Defaults the generator infers:

- The model is the singular of the feature (`notes` -> `Note`), with an `id` and the simple
  fields the events carry; with an HTTP provider, models come from the DTOs (`NoteDto` ->
  `Note`).
- Repository methods come from event verbs: `NotesRequested` -> `list`, `NoteAdded` -> `add`,
  `NoteDeleted` -> `delete`, `NoteUpdated` -> `update`. With an HTTP provider and no
  repository, one method per endpoint.
- A write reloads what the state shows, through the method that loads it.
- A cubit gets a method per event: `NoteAdded` -> `add_note`.

### Running it again

Generation is idempotent: running the same command again changes nothing. A generated file
that you edited is a conflict: nothing is written and the exit code is `2`, unless you pass
`--force`. Models, `DataError` and (for `bloc`) the data layer are only written when missing.

## The JSON spec

`cargo rok-ui g schema` prints the JSON Schema. Unknown keys are errors that name the key.

```text
{
  "feature": "notes",
  "kind": "bloc",                       // or "cubit"
  "name": "Notes",                      // NotesBloc; default: the feature
  "concurrency": "sequential",
  "events": [
    { "name": "NoteAdded", "fields": [{ "name": "title", "type": "String" }],
      "calls": "add", "concurrency": "droppable" }
  ],
  "state": {
    "style": "struct",                  // or "enum" (needs success and failure)
    "status": ["initial", "loading", "success", "failure"],
    "fields": [{ "name": "notes", "type": "Vec<Note>", "default": "Vec::new()" }],
    "error": "ApiError"                 // default: ApiError for HTTP, else DataError
  },
  "models": [{ "name": "Note", "fields": [...] }],
  "repository": {
    "name": "NotesRepository",
    "methods": [
      { "name": "add", "args": [...], "returns": "Note", "calls": "create_note",
        "session": "sign_in", "token_field": "token" }   // session: sign_in | sign_out
    ]
  },
  "provider": {
    "name": "NotesApi", "kind": "http", "api_base": "/api",
    "endpoints": [
      { "method": "create_note", "verb": "POST", "path": "/notes/{id}",
        "path_params": [{ "name": "id", "type": "NoteId" }],
        "query": [{ "name": "search", "type": "Option<String>", "rename": "q" }],
        "body": "NewNoteDto", "raw_body": false,
        "response": "NoteDto",            // or "()", "bytes", "Vec<NoteDto>"
        "skip_expire": false }
    ]
  },
  "dtos": [
    { "name": "NoteDto", "fields": [{ "name": "created_at", "type": "String", "rename": "createdAt" }] },
    { "name": "AttachmentDto", "variants": [{ "name": "Image", "fields": [...] }] }
  ],
  "view": {
    "page": true,
    "form": {
      "event": "NoteAdded",
      "fields": [...],                  // default: the event's fields
      "server_errors": { "field_map": { "noteTitle": "title" }, "conflict_field": "title",
                         "bad_request_field": "title", "unauthorized_field": "password" }
    }
  },
  "tests": true
}
```

Checks (each error names its JSON path): names are valid and not keywords, types parse as
Rust, events are PascalCase and unique, `calls` names an existing method, method arguments
match the event's fields, endpoint paths start with `/` and their `{params}` match
`path_params`, a body is a DTO and not also raw, a `sign_in` response has the token field,
and form fields named by `server_errors` exist. Complete examples:
`rok-ui-cli/fixtures/notes.json`, `tasks.json`, `notes_http.json` and `auth.json`.
