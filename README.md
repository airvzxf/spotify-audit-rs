# Spotify Audit RS 🦀

Una herramienta de auditoría forense y gestión avanzada para bibliotecas de Spotify, escrita en Rust. Diseñada para detectar canciones eliminadas, sincronizar playlists masivas y limpiar duplicados "muertos".

## Características

- **🔍 Auditoría de Integridad**: Detecta canciones "grises" (no reproducibles) en tu biblioteca.
- **🌍 Análisis Forense**: Distingue entre bloqueos regionales (Geo-Locked) y eliminaciones globales.
- **🔄 Sincronización Inteligente**: Mueve playlists enteras a "Liked Songs", recuperando automáticamente versiones obsoletas mediante *Track Relinking*.
- **🧹 Deduplicación (Dedup)**: Elimina automáticamente versiones "muertas" de canciones si ya tienes la versión "viva" (basado en ISRC).
- **📋 Inventario**: Lista tus playlists con detalles de propiedad y colaboración.

## Instalación

Necesitas tener Rust instalado.

```bash
# Compilar el proyecto
cargo build --release
```

## Configuración

Crea un archivo `.env` en la raíz del proyecto con tus credenciales de Spotify Developer:

```env
RSPOTIFY_CLIENT_ID=tu_client_id
RSPOTIFY_CLIENT_SECRET=tu_client_secret
RSPOTIFY_REDIRECT_URI=http://127.0.0.1:8000/callback
```

## Uso

### 1. Escanear Librería (Audit)
Busca canciones rotas en tus "Me Gusta".

```bash
cargo run -p audit-cli -- scan
```

O escanea una playlist específica:
```bash
cargo run -p audit-cli -- scan --playlist <PLAYLIST_ID>
```

### 2. Sincronizar Playlist
Copia todas las canciones de una playlist a tus "Me Gusta". **Detecta y agrega automáticamente las versiones vivas** si las originales están rotas.

```bash
cargo run -p audit-cli -- sync <PLAYLIST_ID>
```

### 3. Inspección Forense
Analiza una canción específica por su ID para ver metadatos ocultos (ISRC, Mercados, Popularidad).

```bash
cargo run -p audit-cli -- inspect <TRACK_ID>
```

### 4. Deduplicación (Limpieza)
Busca en tu librería "Liked Songs" pares de canciones que comparten el mismo ISRC (misma grabación) pero una está "viva" y la otra "muerta", y elimina la muerta.

```bash
cargo run -p audit-cli -- dedup
```

### 5. Listar Playlists
Muestra tus playlists, IDs y si son colaborativas.

```bash
cargo run -p audit-cli -- list
```

## Debugging

Si algo falla, puedes activar los logs detallados:

```bash
RUST_LOG=debug cargo run -p audit-cli -- <COMANDO>
```

## Licencia
GNU Affero General Public License v3 (AGPLv3)