# Bloc de notas con bloques estructurados — Especificación v1

**Autor:** JZ Dev
**Estado:** definitivo para iniciar construcción
**Nombre del proyecto:** pendiente (usar `notas` como nombre de trabajo)
**Stack:** Rust + egui/eframe

---

## 1. Resumen

Un bloc de notas de escritorio, **rápido, liviano y de archivos planos**, con una capacidad extra: dentro del texto se pueden escribir **bloques con notación `@`** que la aplicación interpreta como tablas o registros. Esos bloques se pueden ordenar, filtrar y copiar con un clic, pero el archivo sigue siendo texto plano legible en cualquier otro editor.

**No es un Excel.** No hay fórmulas, celdas direccionables ni hojas. Solo hay texto con pequeñas funciones de tabla.

### Casos de uso que guían el diseño

1. **Lista con fecha:** correos con una fecha al lado, ordenables por la fecha más cercana.
2. **Datos de conexión agrupados:** `ip`, `user`, `pass` escritos en líneas separadas o en una sola línea.
3. **Pares usuario/contraseña.**
4. **Copiado rápido** de cualquier campo o registro completo.

### Principios

- **El texto es la fuente de verdad.** La vista de tabla es una proyección del texto, nunca al revés.
- **Nada se pierde ni se rompe.** Una línea que no se entiende se muestra como texto o con un aviso. Nunca produce error.
- **Los archivos no se reformatean.** Solo se reescriben las líneas que el usuario tocó.
- **Arranque instantáneo y poca memoria.**

---

## 2. Alcance

### Incluido en v1

- Espacio de trabajo con proyectos y notas generales.
- Editor con pestañas y autoguardado.
- Bloques `@tabla`, `@registro` y `@fin`.
- Vista de tabla con copiado rápido, edición de celdas y ordenamiento.
- Campos tipo `texto`, `numero`, `fecha`, `secreto`, `url`, `email`.
- Búsqueda global (paleta `Ctrl+P`).

### Incluido en v1.1

- `@def` (tipos personalizados) y `@lista` (checklists).
- Opciones `filtro`, `alerta`, `total`, `unico`, `colapsado`.
- Atajos de copiado por columna.

### Incluido en v2

- `@ref`, `@secreto`, `@recordatorio`, `@etiqueta`.
- Cifrado de archivos con clave maestra.
- Subcarpetas anidadas dentro de proyectos.

### Fuera de alcance

Fórmulas, referencias entre celdas, gráficos, sincronización en la nube propia, versión móvil o web.

---

## 3. Stack y dependencias

| Necesidad | Crate | Notas |
|---|---|---|
| Ventana y UI | `eframe`, `egui` | Backend `glow` o `wgpu`; preferir `glow` por ligereza |
| Tablas | `egui_extras` | `TableBuilder` |
| Fechas | `chrono` | Parseo de formatos propios |
| Portapapeles | `arboard` | Multiplataforma |
| Recorrer carpetas | `walkdir` | Escaneo del espacio de trabajo |
| Cambios externos | `notify` | Vigilar el espacio de trabajo |
| Configuración | `serde`, `toml`, `directories` | `config.toml` en la carpeta de configuración del sistema |
| Errores | `thiserror`, `anyhow` | `thiserror` en `core`, `anyhow` en `app` |
| Búsqueda difusa | `nucleo-matcher` | Paleta de comandos |
| Tests | `proptest`, `insta` | Propiedades y snapshots |
| v2: cifrado | `argon2`, `chacha20poly1305`, `zeroize` | Clave maestra y limpieza de memoria |

### Perfil de compilación

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
```

---

## 4. Estructura en disco (datos del usuario)

```
MisNotas/                   ← espacio de trabajo (cualquier carpeta)
├─ general/                 ← notas sin proyecto
│   ├─ ideas.txt
│   └─ pendientes.txt
├─ servidores/              ← proyecto = subcarpeta
│   ├─ ftp.txt
│   ├─ correos.txt
│   └─ proyecto.toml        ← opcional: color, icono, orden
├─ tesis/
│   └─ referencias.md
└─ .app/                    ← caché e índice (se puede borrar sin perder nada)
```

### Reglas

- **Espacio de trabajo** = carpeta elegida por el usuario. Se recuerda en `config.toml`.
- **Proyecto** = subcarpeta directa del espacio de trabajo, excepto `general` y las que empiezan con `.`.
- **Nota** = archivo `.txt` o `.md` dentro de un proyecto o de `general`.
- **Notas generales** = archivos en `general/`. Si el usuario deja archivos sueltos en la raíz, también se tratan como generales.
- **Una nota pertenece a un solo proyecto** (el de su carpeta). Las conexiones entre proyectos se resuelven con `@etiqueta` y `@ref` (v2).
- Los archivos y carpetas ocultos y la carpeta `.app/` se ignoran.
- Para v1, solo un nivel: `proyecto/nota`. Las subcarpetas dentro de un proyecto se ignoran y se habilitan en v2.

### `proyecto.toml` (opcional)

```toml
nombre = "Servidores"      # sobrescribe el nombre de la carpeta
color  = "#4f8cff"
icono  = "🖥"
orden  = 2                 # posición en la barra lateral
```

---

## 5. Arquitectura del código

Un workspace de Cargo con tres crates:

```
notas/
├─ Cargo.toml                 ← [workspace] members = ["core", "storage", "app"]
├─ core/                      ← lógica pura, sin UI ni disco
│   └─ src/
│       ├─ lib.rs
│       ├─ lexer.rs           ← detecta líneas @ y las clasifica
│       ├─ header.rs          ← parsea la cabecera de un bloque
│       ├─ schema.rs          ← Schema, Field, FieldType
│       ├─ options.rs         ← Options (orden, copiar, sep, modo, vista…)
│       ├─ block.rs           ← Block, Row, Layout
│       ├─ parse.rs           ← texto → segmentos y bloques
│       ├─ serialize.rs       ← bloques → texto (solo líneas tocadas)
│       ├─ dates.rs           ← parseo de fechas y distancia a hoy
│       ├─ template.rs        ← expansión de {campo}
│       └─ ops.rs             ← ordenar, filtrar, editar celda, agregar fila
├─ storage/                   ← disco y estado del espacio de trabajo
│   └─ src/
│       ├─ workspace.rs       ← escaneo y árbol (general + proyectos)
│       ├─ note.rs            ← cargar/guardar, EOL, codificación
│       ├─ autosave.rs        ← debounce y escritura atómica
│       ├─ watcher.rs         ← cambios externos
│       ├─ index.rs           ← índice de búsqueda en memoria
│       └─ config.rs          ← config.toml
└─ app/                       ← egui/eframe
    └─ src/
        ├─ main.rs
        ├─ state.rs           ← estado global de la aplicación
        ├─ sidebar.rs         ← árbol: General + Proyectos
        ├─ tabs.rs            ← notas abiertas
        ├─ editor.rs          ← render de segmentos
        ├─ text_segment.rs    ← TextEdit con resaltado de líneas @
        ├─ block_view.rs      ← tabla/registro/lista
        ├─ palette.rs         ← Ctrl+P
        ├─ clipboard.rs       ← copiado y limpieza automática
        └─ theme.rs
```

### Regla de dependencias

`app → storage → core`. El crate `core` no depende de egui, ni del disco, ni de `chrono` más allá de lo necesario para fechas. Casi toda la lógica de negocio y casi todos los tests viven en `core`.

---

## 6. Modelo de datos

### 6.1 Espacio de trabajo

```rust
pub struct Workspace {
    pub root: PathBuf,
    pub general: Vec<NoteRef>,
    pub projects: Vec<Project>,
}

pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub meta: ProjectMeta,       // de proyecto.toml, con valores por defecto
    pub notes: Vec<NoteRef>,
}

pub struct NoteRef {
    pub path: PathBuf,
    pub title: String,           // nombre de archivo sin extensión
    pub modified: SystemTime,
}
```

### 6.2 Nota abierta

El editor de egui (`TextEdit`) trabaja sobre `String`, no sobre ropes. Por eso la nota abierta se divide en **segmentos**, cada uno con su propio `String`:

```rust
pub struct Note {
    pub path: PathBuf,
    pub eol: Eol,                // Lf | CrLf, detectado al cargar
    pub segments: Vec<Segment>,
    pub dirty: bool,
    pub disk_mtime: SystemTime,
}

pub enum Segment {
    Text(TextSegment),           // texto libre
    Block(BlockSegment),         // cabecera + cuerpo + @fin
}

pub struct TextSegment {
    pub text: String,
}

pub struct BlockSegment {
    pub raw: String,             // texto exacto del bloque, incluida la cabecera
    pub parsed: ParsedBlock,     // derivado de raw
    pub state: ViewState,        // Tabla | Crudo (no se guarda en el archivo)
}
```

El texto completo de la nota es la concatenación de `raw`/`text` de todos los segmentos, unidos con el salto de línea original. **No se usa `ropey` en v1.** Si aparecen notas de más de ~1 MB, se evalúa.

### 6.3 Bloques

```rust
pub struct ParsedBlock {
    pub kind: BlockKind,
    pub title: Option<String>,
    pub schema: Schema,
    pub options: Options,
    pub layout: Layout,          // Fila | Vertical, ya resuelto
    pub rows: Vec<Row>,
    pub issues: Vec<Issue>,      // líneas que no encajan, con su número
    pub closed: Closing,         // Fin | SiguienteBloque | FinDeArchivo
}

pub enum BlockKind {
    Tabla,
    Registro,
    Lista,                       // v1.1
    Custom(String),              // definido con @def (v1.1)
}

pub struct Schema { pub fields: Vec<Field> }
pub struct Field  { pub name: String, pub ty: FieldType }

pub enum FieldType { Text, Number, Date, Secret, Url, Email, Check }

pub struct Row {
    pub cells: Vec<String>,      // un valor por campo
    pub src_lines: Range<usize>, // líneas del cuerpo que originan la fila
}

pub struct Issue { pub line: usize, pub message: String }
```

### 6.4 Invariante de ida y vuelta

> Para cualquier bloque que el usuario **no modificó**, `serialize(parse(texto)) == texto`, byte a byte.

Cuando el usuario edita una celda, ordena o agrega una fila, solo se reescriben las líneas afectadas. Todo lo demás (espacios, comentarios, líneas desconocidas) se conserva.

---

## 7. Especificación de la notación `@`

### 7.1 Reglas generales de detección

1. Un comando solo se reconoce si `@` es el **primer carácter de la línea** (sin espacios antes).
2. El nombre del comando debe ser **conocido** (`tabla`, `registro`, `fin`, etc., o un tipo definido con `@def`). Un `@` seguido de un nombre desconocido es **texto normal**, sin error.
3. Un `@` en medio de una línea nunca se interpreta. Por eso `test@gmail.com` y `@juan` son seguros.
4. Para escribir como texto una línea que empieza con un comando, se antepone `\`. Ejemplo: `\@tabla` se muestra como `@tabla`.
5. `@//` al inicio de línea es un **comentario**. Se ignora en la vista de tabla y se conserva en el archivo.

### 7.2 Clases de comandos

| Clase | Descripción |
|---|---|
| **Bloque** | Tiene cuerpo. Se cierra con `@fin`, con la siguiente cabecera de bloque o con el fin del archivo. |
| **Línea** | Ocupa una sola línea y no tiene cuerpo. |

### 7.3 Catálogo de comandos

| Comando | Clase | Para qué sirve | Versión |
|---|---|---|---|
| `@tabla` | bloque | Lista con columnas; se ordena y filtra | v1 |
| `@registro` | bloque | Datos agrupados; se copian | v1 |
| `@fin` | línea | Cierra un bloque | v1 |
| `@//` | línea | Comentario | v1 |
| `@def` | línea | Define una plantilla de bloque reutilizable | v1.1 |
| `@lista` | bloque | Checklist `[ ]` / `[x]` | v1.1 |
| `@ref` | línea | Enlaza o inserta un bloque de otra nota | v2 |
| `@secreto` | bloque | Texto oculto o cifrado | v2 |
| `@recordatorio` | línea | Fecha con aviso | v2 |
| `@etiqueta` | línea | Etiquetas para búsqueda | v2 |

### 7.4 Cabecera de bloque

```
@tipo ["título"] [campo[:tipo], campo[:tipo], ...] [| opcion=valor | opcion=valor ...]
```

Ejemplo:

```
@registro "FTP oficina" ip, user, pass:secreto | orden=ip | copiar=ftp://{user}:{pass}@{ip}
192.168.1.10 admin 1234
10.0.0.5 root abcd
@fin
```

Reglas:

- El **título** va entre comillas dobles y es opcional.
- Los **campos** se separan con comas. Cada uno puede llevar `:tipo`. Sin tipo, es `texto`.
- Las **opciones** van después de `|` y se separan entre sí con `|`. Si un valor contiene `|`, debe ir entre comillas dobles.
- Si un bloque `@tabla` o `@registro` no declara campos, se infiere una columna por token de la primera fila, con nombres `col1`, `col2`, … (los campos se pueden nombrar después).

### 7.5 Tipos de campo

| Tipo | Valida | Orden | Vista |
|---|---|---|---|
| `texto` | nada | alfabético sin distinguir mayúsculas | texto |
| `numero` | número con `.` o `,` decimal | numérico | alineado a la derecha |
| `fecha` | ver 7.9 | cronológico o por cercanía | fecha normalizada |
| `secreto` | nada | — | `••••`, se revela con clic |
| `url` | empieza con `http(s)://` o `ftp://` | alfabético | enlace clicable |
| `email` | contiene un solo `@` y un punto en el dominio | alfabético | texto |
| `check` | `[ ]`, `[x]`, `si`, `no` | por estado | casilla |

**Secretos automáticos:** un campo sin tipo explícito cuyo nombre sea `pass`, `password`, `contra`, `contraseña`, `clave`, `token`, `secret` o `pin` se trata como `secreto`. La lista es configurable.

### 7.6 Opciones

**v1**

| Opción | Valores | Efecto |
|---|---|---|
| `orden` | `campo`, `campo desc`, `campo cercana` | Ordena y **reescribe las líneas del cuerpo**. `cercana` (solo fechas) pone primero las más próximas a hoy, futuras antes que pasadas |
| `copiar` | plantilla con `{campo}` | Plantilla para copiar el registro completo |
| `sep` | `espacio` (defecto), `tab`, `\|`, `,`, `;` | Separador de campos en modo fila |
| `modo` | `auto` (defecto), `fila`, `vertical` | Cómo se agrupan las líneas en registros |
| `vista` | `tabla` (defecto), `crudo` | Vista por defecto |

**v1.1**

| Opción | Efecto |
|---|---|
| `filtro=campo~texto` | Muestra solo filas que contienen el texto. Es solo de vista, no reescribe |
| `alerta=campo<7d` | Resalta fechas dentro de los próximos 7 días (`d`, `w`, `m`) |
| `total=campo` | Suma una columna numérica y la muestra al pie |
| `unico=campo` | Marca valores duplicados |
| `colapsado` | El bloque empieza plegado |

Las opciones desconocidas se ignoran y se muestran como aviso en la cabecera del bloque.

### 7.7 Cierre de bloques

Un bloque termina en el **primero** de estos eventos:

1. Una línea `@fin`.
2. La siguiente cabecera de bloque (`@tabla`, `@registro`, etc.).
3. El fin del archivo.

El editor **inserta `@fin` automáticamente** cuando el usuario escribe una cabecera de bloque y presiona Enter. Un `@fin` sin bloque abierto se trata como texto y genera un aviso.

Dentro del cuerpo de un bloque:

- Las líneas `@//` son comentarios y se ignoran.
- Cualquier otra línea que empiece con `@` y sea un comando de bloque conocido cierra el bloque anterior.
- Para que una línea de datos empiece con `@`, se escapa con `\@`.

### 7.8 `@def` (v1.1)

Define un tipo de bloque reutilizable:

```
@def ftp ip, user, pass:secreto | copiar=ftp://{user}:{pass}@{ip}

@ftp "Oficina"
192.168.1.10 admin 1234
@fin

@ftp "Casa" | orden=ip
10.0.0.5 root abcd
@fin
```

- Un tipo definido se comporta como `@registro` con ese esquema y esas opciones.
- Las opciones dadas en el uso **sobrescriben** las del `@def`.
- Alcance: **por nota**. En v2 se podrá definir a nivel de espacio de trabajo.
- Si el nombre coincide con un comando integrado, el integrado gana y se muestra un aviso.

### 7.9 Fechas

Formatos aceptados:

| Formato | Ejemplo | Notas |
|---|---|---|
| `dd/mm` | `12/11` | Sin año: se asume el año actual |
| `dd/mm/aaaa` | `12/11/2026` | |
| `dd-mm-aaaa` | `12-11-2026` | |
| `aaaa-mm-dd` | `2026-11-12` | ISO |

- El orden día/mes es configurable (`fecha_orden = "dmy"` por defecto).
- Una fecha inválida no rompe el bloque: la celda se marca con aviso y se ordena al final.
- Para `orden=campo cercana`, la distancia se mide contra la fecha de hoy.

### 7.10 Plantillas de copiado

- `{campo}` se reemplaza por el valor del campo. Ejemplo: `ftp://{user}:{pass}@{ip}`.
- `{{` y `}}` producen llaves literales.
- Un campo inexistente deja el texto `{campo}` y genera un aviso en la cabecera.
- Sin `copiar=`, copiar el registro entrega los valores separados por tabulador.

### 7.11 Gramática simplificada

```
documento   = { linea } ;
linea       = comentario | cabecera | fin | def | texto ;
comentario  = "@//" { cualquier } ;
cabecera    = "@" nombre [ ws titulo ] [ ws campos ] [ ws "|" opciones ] ;
fin         = "@fin" ;
def         = "@def" ws nombre ws campos [ ws "|" opciones ] ;
titulo      = '"' { cualquier-menos-comilla } '"' ;
campos      = campo { "," ws campo } ;
campo       = ident [ ":" tipo ] ;
opciones    = opcion { "|" opcion } ;
opcion      = ident "=" valor ;
texto       = [ "\" ] cualquier ;
```

---

## 8. Parser

### 8.1 Pipeline

```
texto del archivo
   │
   ▼
lexer      → clasifica cada línea: Texto | Cabecera | Fin | Def | Comentario
   │
   ▼
segmentador → agrupa líneas en Segmentos (Texto / Bloque), resuelve el cierre
   │
   ▼
header     → parsea cabecera de cada bloque: tipo, título, campos, opciones
   │
   ▼
cuerpo     → detecta el layout y arma las filas
   │
   ▼
ParsedBlock (con issues)
```

### 8.2 Detección de layout (`modo=auto`)

Sea **N** el número de campos del esquema y las *líneas de datos* las del cuerpo sin comentarios ni líneas en blanco.

1. Si `N == 1`: modo **fila**.
2. Si la **mayoría** de las líneas de datos tiene exactamente **un** token: modo **vertical**.
3. En otro caso: modo **fila**.

### 8.3 Modo fila

- Se divide cada línea según `sep`.
- Con `sep=espacio`, se aceptan tokens entre comillas dobles: `"Juan Pérez" juan@mail.com`.
- Si hay **más** tokens que campos, el **último campo absorbe el resto** de la línea. Así una contraseña con espacios no rompe nada.
- Si hay **menos** tokens que campos, la línea se conserva, se marca como `Issue` y las celdas faltantes quedan vacías.

### 8.4 Modo vertical

Cada línea de datos es **un campo completo** (no se dividen por espacios).

1. Se intenta agrupar por **líneas en blanco**. Si todos los grupos tienen exactamente N líneas, se usa esa agrupación.
2. Si no, se agrupa de a **N líneas consecutivas** ignorando las líneas en blanco.
3. Un último grupo incompleto se conserva con las celdas faltantes vacías y un `Issue`.

### 8.5 Errores y avisos

Ningún problema impide que el bloque se muestre. Los `Issue` se pintan como un icono amarillo en la fila y un resumen en la cabecera del bloque. Las líneas problemáticas permanecen intactas en el texto.

### 8.6 Reparseo incremental

- Cada segmento se reparsea **solo cuando su texto cambia**.
- Después de editar un segmento, se vuelve a ejecutar el lexer sobre ese segmento y sus vecinos inmediatos, porque escribir o borrar una cabecera puede crear, partir o fusionar bloques.
- Un reparseo de un bloque normal debe tardar menos de 1 ms.

### 8.7 Serialización mínima

- **Editar una celda:** se reescribe solo la línea (modo fila) o las líneas de ese campo (modo vertical) de esa fila. Los demás campos conservan su texto original.
- **Agregar fila:** se inserta al final del cuerpo, antes de `@fin`, con el formato del bloque.
- **Ordenar:** se permutan las líneas (o los grupos de líneas) sin modificar su contenido. Los comentarios y las líneas no reconocidas se quedan **pegados a la fila que les sigue**.
- Si un valor contiene espacios y el modo es fila con `sep=espacio` y no es el último campo, se escribe entre comillas dobles.

---

## 9. Operaciones

| Operación | Reescribe texto | Descripción |
|---|---|---|
| Ordenar | Sí | Ordenamiento **estable**. Vacíos y valores inválidos van al final |
| Filtrar | No | Solo cambia lo que se ve (v1.1) |
| Editar celda | Sí | Solo la(s) línea(s) afectadas |
| Agregar fila | Sí | Antes de `@fin` |
| Eliminar fila | Sí | Quita sus líneas y los comentarios pegados a ellas |
| Copiar celda | No | Valor de la celda |
| Copiar registro | No | Plantilla `copiar=` o valores separados por tabulador |

Ordenar desde la cabecera de la tabla (clic en una columna) actualiza también la opción `orden=` de la cabecera `@` para que el orden sea reproducible. Un segundo clic alterna `desc`.

---

## 10. Interfaz

### 10.1 Distribución

```
┌──────────────┬───────────────────────────────────────────┐
│ 🔍 Buscar    │  [ftp.txt] [correos.txt] [ideas.txt] [+]  │
│              ├───────────────────────────────────────────┤
│ ▾ General    │                                           │
│    ideas     │   texto libre…                            │
│    pendientes│                                           │
│ ▾ Servidores │   ┌─ registro · FTP oficina ───────────┐  │
│    ftp       │   │ ip           user     pass          │  │
│    correos   │   │ 192.168.1.10 admin    ••••   [📋]   │  │
│ ▸ Tesis      │   │ 10.0.0.5     root     ••••   [📋]   │  │
│              │   └─────────────────────────────────────┘  │
│              │   más texto libre…                         │
└──────────────┴───────────────────────────────────────────┘
```

- **Barra lateral:** `General` arriba, luego los proyectos plegables. Ancho ajustable y ocultable.
- **Pestañas:** varias notas abiertas. Un punto en la pestaña indica cambios sin guardar.
- **Centro:** la nota como secuencia de segmentos.

### 10.2 Estados de un bloque

Cada bloque tiene dos estados (no se guardan en el archivo):

- **Tabla:** el bloque se dibuja como tabla.
- **Crudo:** se edita el texto del bloque.

Transiciones:

- Clic en `[</>]`, o mover el cursor al bloque con el teclado → **Crudo**.
- El cursor sale del bloque, o `Esc` → **Tabla** (se reparsea).
- La opción `vista=crudo` hace que el bloque empiece en Crudo.

### 10.3 Interacciones en la tabla

| Acción | Resultado |
|---|---|
| Clic en una celda | Copia su valor y muestra un aviso breve |
| Botón `[📋]` de la fila | Copia el registro completo |
| Doble clic | Edita la celda; al terminar se reescribe la línea |
| Clic en el encabezado de una columna | Ordena (alterna ascendente/descendente) |
| Clic en `••••` | Revela el secreto mientras el cursor esté encima o por unos segundos |
| `[+]` | Agrega una fila |
| `[</>]` | Cambia a Crudo |
| Flechas | Mueven la selección de celda |
| `Enter` | Edita la celda seleccionada |
| `Ctrl+C` | Copia la celda seleccionada |
| `Supr` | Elimina la fila seleccionada (con confirmación) |

### 10.4 Edición de texto libre

- Cada segmento de texto es un `TextEdit::multiline` sin borde, con altura que se ajusta al contenido.
- Las líneas que empiezan con un comando reconocido se resaltan con color distinto.
- **Navegación entre segmentos:** al llegar al borde de un segmento con las flechas arriba/abajo, el foco pasa al segmento vecino. Es la parte más delicada de la implementación en egui y debe resolverse en la Fase 3.

### 10.5 Atajos globales

| Atajo | Acción |
|---|---|
| `Ctrl+S` | Guardar ahora |
| `Ctrl+N` | Nueva nota en el proyecto activo (o en General) |
| `Ctrl+P` | Paleta: notas y registros |
| `Ctrl+F` | Buscar dentro de la nota |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cambiar de pestaña |
| `Ctrl+W` | Cerrar pestaña |
| `Ctrl+B` | Mostrar/ocultar barra lateral |
| `Ctrl+Enter` | Insertar bloque nuevo (menú de tipos) |
| `Esc` | Salir del modo Crudo / cerrar paleta |
| `Alt+1`…`Alt+9` (v1.1) | Con una fila seleccionada, copia su campo n |

### 10.6 Paleta `Ctrl+P`

- Una sola caja de búsqueda sobre el índice global.
- Resultados agrupados: **Notas** (por nombre) y **Registros** (por contenido de filas).
- `Enter` abre la nota y posiciona el bloque. `Ctrl+Enter` copia el registro directamente sin abrir la nota.
- Los campos secretos **no** se indexan para búsqueda de contenido.

---

## 11. Persistencia

### 11.1 Archivos

- Codificación **UTF-8**. Al abrir un archivo con bytes inválidos se muestra un aviso y se abre en modo de solo lectura.
- Se detecta el fin de línea (`LF` o `CRLF`) al cargar y se **conserva** al guardar.
- Extensiones aceptadas: `.txt` y `.md`. Las notas nuevas se crean como `.txt`.

### 11.2 Autoguardado

- Guarda **800 ms** después de la última modificación (configurable).
- También guarda al cambiar de pestaña, al cerrar la nota y al cerrar la aplicación.
- Escritura **atómica**: se escribe a un archivo temporal en la misma carpeta y luego se renombra.

### 11.3 Cambios externos

- `notify` vigila el espacio de trabajo.
- Si una nota abierta **sin cambios locales** cambia en disco, se recarga en silencio.
- Si una nota abierta **con cambios locales** cambia en disco, se muestra una barra con tres opciones: *Recargar* (pierde lo mío), *Conservar lo mío* (sobrescribe) o *Guardar como copia*.
- Antes de cada guardado se compara `mtime` para detectar el conflicto.

### 11.4 Configuración (`config.toml`)

```toml
workspace        = "C:/Users/.../MisNotas"
autosave_ms      = 800
fecha_orden      = "dmy"
clipboard_clear  = 30            # segundos; 0 desactiva
secret_names     = ["pass", "password", "contra", "contraseña", "clave", "token", "secret", "pin"]
font_size        = 15
theme            = "system"      # system | light | dark
```

---

## 12. Búsqueda e índice

- Al abrir el espacio de trabajo se carga solo la **lista de archivos**. El contenido de cada nota se lee cuando se abre.
- En segundo plano se construye un índice en memoria: nombre de nota, proyecto, títulos de bloque, filas (sin campos secretos).
- El índice se guarda en `.app/index.bin` y se actualiza de forma incremental con `notify` y `mtime`.
- La búsqueda difusa usa `nucleo-matcher`.
- Meta: resultados en menos de 50 ms con 5 000 notas.

---

## 13. Seguridad

### v1

- Los campos `secreto` se muestran como `••••` por defecto.
- Al copiar un secreto, el portapapeles se limpia a los `clipboard_clear` segundos, **solo si** el contenido sigue siendo el mismo que se copió.
- Los secretos no se indexan ni aparecen en las vistas previas de la paleta.
- **Los archivos siguen en texto plano.** Esto debe quedar claro para el usuario. Se muestra una nota única en el primer uso.

### v2

- Cifrado por archivo o por proyecto con clave maestra:
  - Derivación: `argon2id`.
  - Cifrado: `ChaCha20-Poly1305`.
  - Extensión `.txt.enc`.
- Limpieza de memoria con `zeroize` en las claves y en los secretos en claro.
- Bloqueo automático tras N minutos de inactividad.

---

## 14. Rendimiento

| Métrica | Meta |
|---|---|
| Arranque hasta ventana usable | < 150 ms |
| Memoria en reposo | < 50 MB |
| Tamaño del binario (release) | < 15 MB |
| Abrir una nota de 1 MB | < 100 ms |
| Reparseo de un bloque | < 1 ms |
| Búsqueda en 5 000 notas | < 50 ms |
| Uso de CPU en reposo | ~0 % (repaint solo bajo demanda) |

Prácticas:

- Repintar solo cuando hay eventos (`ctx.request_repaint()` bajo demanda).
- Tablas con virtualización (`TableBuilder` dibuja solo las filas visibles).
- Sin asignaciones por cuadro en la ruta de dibujo.
- Escaneo, indexado y vigilancia en hilos secundarios, comunicados por canales.

---

## 15. Pruebas

### Unitarias (en `core`)

- **Lexer:** líneas con `@` al inicio, en medio, escapadas, desconocidas, correos.
- **Cabecera:** con y sin título, campos, tipos, opciones, comillas y `|` dentro de valores.
- **Layouts:** fila, vertical por líneas en blanco, vertical por N líneas, auto, grupos incompletos.
- **Fechas:** todos los formatos, años omitidos, fechas inválidas, orden `cercana`.
- **Plantillas:** campos válidos, inexistentes, llaves escapadas.

### Propiedades (`proptest`)

- **Ida y vuelta:** para texto arbitrario, `serialize(parse(t)) == t` en bloques sin modificar.
- **Ordenar:** el resultado es una permutación de las filas originales; ordenar dos veces da lo mismo.
- **Editar celda:** solo cambian las líneas de esa fila.
- **Robustez:** el parser nunca entra en pánico con cualquier entrada.

### Snapshots (`insta`)

Un conjunto de notas de ejemplo (correos con fechas, FTP vertical y en fila, pares user/pass, bloques sin cerrar, `@` escapados) con su `ParsedBlock` esperado.

### Integración

- Abrir, editar y guardar con `LF` y `CRLF`.
- Conflicto con cambio externo.
- Espacio de trabajo con proyectos y notas generales.

---

## 16. Hoja de ruta

### Fase 0 — Preparación
- [ ] Crear el workspace de Cargo (`core`, `storage`, `app`).
- [ ] Configurar `rustfmt`, `clippy` y CI de pruebas.
- [ ] Crear el conjunto de notas de ejemplo para pruebas.

### Fase 1 — Base de notas
- [ ] `config.toml` y selección del espacio de trabajo.
- [ ] Escaneo de `general/` y proyectos.
- [ ] Barra lateral con árbol.
- [ ] Pestañas y editor de texto simple.
- [ ] Guardado atómico y autoguardado.
- [ ] Detección de `LF`/`CRLF`.

### Fase 2 — Parser (`core`)
- [ ] Lexer y segmentador.
- [ ] Cabeceras, esquemas, tipos y opciones.
- [ ] Layouts fila y vertical, modo auto.
- [ ] Fechas y plantillas.
- [ ] Serialización mínima y prueba de ida y vuelta.
- [ ] Batería completa de pruebas.

### Fase 3 — Bloques en la interfaz
- [ ] Modelo de segmentos en el editor.
- [ ] Navegación del cursor entre segmentos.
- [ ] Render de tabla con `TableBuilder`.
- [ ] Estados Tabla/Crudo y transiciones.
- [ ] Resaltado de líneas `@`.
- [ ] Inserción automática de `@fin`.

### Fase 4 — Interacción con datos
- [ ] Copiado de celda y registro, con aviso.
- [ ] Campos secretos y limpieza del portapapeles.
- [ ] Edición de celdas con reescritura mínima.
- [ ] Agregar y eliminar filas.
- [ ] Ordenar por columna, incluida `cercana` en fechas.
- [ ] Avisos de líneas con problemas.

### Fase 5 — Búsqueda
- [ ] Índice en memoria y en `.app/`.
- [ ] Paleta `Ctrl+P` con notas y registros.
- [ ] Búsqueda dentro de la nota (`Ctrl+F`).
- [ ] Vigilancia de cambios externos y manejo de conflictos.

**Fin de v1.**

### Fase 6 — v1.1
- [x] `@def` y tipos personalizados.
- [x] `@lista`.
- [x] Opciones `filtro`, `alerta`, `total`, `unico`, `colapsado`.
- [x] Detección heurística de pares `clave : valor` sin `@`.
- [ ] Atajos `Alt+1`…`Alt+9`.

### Fase 7 — v2
- [ ] Cifrado con clave maestra.
- [ ] `@ref`, `@secreto`, `@recordatorio`, `@etiqueta`.
- [ ] Subcarpetas dentro de proyectos.
- [ ] Definiciones a nivel de espacio de trabajo.

---

## 17. Decisiones pendientes

Estas decisiones tienen un valor por defecto asumido en esta especificación y se pueden cambiar sin rehacer el diseño:

| Tema | Valor asumido | Alternativa |
|---|---|---|
| Cierre de bloques | `@fin` explícito, con inserción automática en el editor; también cierra la siguiente cabecera o el fin del archivo | Cerrar con doble línea en blanco |
| `@def` | Entra en v1.1 | Adelantarlo a v1 |
| Pertenencia de notas | Una nota, un proyecto (carpeta) | Etiquetas para pertenecer a varios |
| Formato de fecha | `dd/mm` | Configurable a `mm/dd` |
| Fecha sin año | Año actual | Próxima ocurrencia |
| Extensión de nota nueva | `.txt` | `.md` |
| Nombre del proyecto | `notas` (de trabajo) | Por definir |

---

## 18. Ejemplo completo de una nota

```
Servidores de la oficina — actualizado en noviembre

@//  datos de acceso, no compartir
@def ftp ip, user, pass:secreto | copiar=ftp://{user}:{pass}@{ip}

@ftp "Oficina"
192.168.1.10 admin 1234
10.0.0.5 root abcd
@fin

Claves en formato vertical:

@registro "Router" ip, user, pass | modo=vertical
192.168.0.1
admin
router2026

192.168.0.2
admin
repetidor
@fin

Correos por vencer:

@tabla "Correos" correo:email, vence:fecha | orden=vence cercana
test@gmail.com 12/11
ana@gmail.com 03/12
luis@gmail.com 28/10
@fin

Línea escapada: \@tabla no abre un bloque.
Un correo normal como juan@mail.com nunca se interpreta.
```

---

## 19. Glosario

| Término | Significado |
|---|---|
| Espacio de trabajo | Carpeta raíz que contiene `general/` y los proyectos |
| Proyecto | Subcarpeta que agrupa notas relacionadas |
| Nota | Archivo `.txt` o `.md` |
| Segmento | Trozo de una nota abierta: texto libre o un bloque |
| Bloque | Texto delimitado por una cabecera `@` y su cierre |
| Esquema | Lista de campos (con tipo) de un bloque |
| Layout | Forma de agrupar las líneas en registros: fila o vertical |
| Crudo | Estado de un bloque en el que se edita su texto |
