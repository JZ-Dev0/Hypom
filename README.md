# Hypom 📝

**Bloc de notas de escritorio, rápido, liviano y de archivos planos con soporte para bloques estructurados.**

Hypom permite escribir texto libre e intercalar bloques estructurados con notación `@` (`@tabla`, `@registro`) que se visualizan como tablas interactivas. El archivo en disco sigue siendo siempre texto plano legible en cualquier editor.

---

## ✨ Características Principales

- **Texto plano como fuente de verdad**: Todo se almacena en archivos `.txt` o `.md`. La tabla es una proyección del texto.
- **Bloques estructurados**:
  - `@tabla`: Listas tabulares con columnas tipadas, ordenamiento y edición rápida.
  - `@registro`: Pares y grupos de datos (credenciales, configuraciones, conexiones).
  - Vistas duales: Alterna al instante entre **Tabla** y modo **Crudo (Texto)** con `[</>]`.
- **Tipos de campos**:
  - `texto`, `numero`, `fecha` (con orden cronológico o por cercanía a hoy), `secreto`, `url`, `email`.
  - Detección automática de campos secretos (`pass`, `password`, `clave`, `token`, etc.).
- **Persistencia inteligente y segura**:
  - Autoguardado atómico con debounce de 800 ms.
  - Detección y conservación estricta de saltos de línea originales (`LF` o `CRLF`).
  - Detección de cambios externos en segundo plano con `notify`.
  - Limpieza automática del portapapeles tras copiar campos secretos.
- **Búsqueda difusa global (`Ctrl + P`)**:
  - Búsqueda instantánea de notas y filas de datos (excluyendo automáticamente campos secretos del índice).
  - Copiado directo desde la paleta con `Ctrl + Enter`.
- **Estructura jerárquica**:
  - Carpeta `general/` para notas sueltas.
  - Subcarpetas como **Proyectos**, configurables con `proyecto.toml` (nombre, icono, color y orden).

---

## 📌 Ejemplo de Nota

```text
Servidores de la oficina

@// Credenciales protegidas
@registro "FTP oficina" ip, user, pass:secreto | orden=ip | copiar=ftp://{user}:{pass}@{ip}
192.168.1.10 admin 1234
10.0.0.5 root abcd
@fin

Correos por vencer:

@tabla "Vencimientos" correo:email, vence:fecha | orden=vence cercana
test@gmail.com 12/11
ana@gmail.com 03/12
luis@gmail.com 28/10
@fin
```

---

## ⌨️ Atajos de Teclado

| Atajo | Acción |
|---|---|
| `Ctrl + P` | Paleta de búsqueda rápida (notas y registros) |
| `Ctrl + S` | Guardar cambios inmediatamente |
| `Ctrl + N` | Crear una nueva nota |
| `Ctrl + B` | Mostrar / ocultar barra lateral |
| `Ctrl + W` | Cerrar pestaña activa |
| `Ctrl + Tab` / `Ctrl + Shift + Tab` | Alternar entre pestañas abiertas |
| `Ctrl + Enter` | Insertar plantilla de bloque estructurado |
| `Clic en celda` | Copia el valor al portapapeles |
| `Doble clic en celda` | Edición inline de la celda |

---

## 🏗 Arquitectura del Código

El proyecto está organizado como un workspace de Cargo con tres crates:

```
HYPOM/
├─ core/       # Lógica pura (lexer, parser, layouts, dates, templates, ops)
├─ storage/    # Persistencia (disco, workspace, autoguardado, watcher, índice)
└─ app/        # Interfaz gráfica de escritorio con egui / eframe
```

---

## 🚀 Compilación y Ejecución

### Requisitos
- **Rust 1.80+** (edición 2021)

### Ejecutar en desarrollo
```bash
cargo run --bin hypom-app
```

### Ejecutar pruebas
```bash
cargo test --workspace
```

### Compilar para producción (Release)
```bash
cargo build --release --bin hypom-app
```
El binario optimizado se encontrará en `target/release/hypom-app.exe`.

---

## 📄 Licencia

Este proyecto está bajo la Licencia MIT. Consulta el archivo [LICENSE](LICENSE) para más detalles.

Desarrollado por **JZ Dev**.

