# KAMUSIC

KAMUSIC es un reproductor de musica local para Ubuntu/GNOME creado con Rust, GTK4, Libadwaita y GStreamer.

## Funciones incluidas

- Ventana principal GTK4/Libadwaita.
- Escaneo de carpetas de musica sin bloquear la interfaz.
- Deteccion recursiva de `.mp3`, `.flac`, `.ogg`, `.opus`, `.wav`, `.m4a` y `.aac`.
- Organizacion principal por carpetas reales del sistema.
- Busqueda por titulo, ruta, carpeta, artista y album.
- Reproduccion con GStreamer mediante `playbin`.
- Controles de reproducir, pausar, detener, anterior, siguiente y volumen.
- Caratulas por archivos `cover`, `folder`, `front` o `album` en la misma carpeta.
- Indice SQLite local y configuracion JSON en rutas XDG.
- Archivos iniciales para empaquetado Snap.

## Dependencias de desarrollo en Ubuntu 26.04

```bash
sudo apt install build-essential cargo rustc pkg-config libgtk-4-dev libadwaita-1-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
sudo apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly
```

## Ejecutar

```bash
cargo run
```

Al abrir la app, usa `Escanear musica` para intentar cargar la carpeta XDG de musica o `Seleccionar carpeta` para elegir una ruta manualmente.

## Empaquetar como Snap

```bash
snapcraft
sudo snap install kamusic_*.snap --dangerous
```

Para musica en discos externos puede ser necesario conectar la interfaz `removable-media`:

```bash
sudo snap connect kamusic:removable-media
```

## Datos locales

KAMUSIC guarda la configuracion, indice y cache mediante rutas XDG de usuario:

- Configuracion: `~/.config/org.kampos.kamusic/settings.json`
- Indice: `~/.local/share/org.kampos.kamusic/library.sqlite3`
- Cache: `~/.cache/org.kampos.kamusic/covers`
