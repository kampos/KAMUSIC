Aquí tienes el `AGENT.md` para pegar directamente en Codex.

````md
# AGENT.md

## Objetivo del proyecto

Crear una aplicación de escritorio para Ubuntu 26.04 llamada **KAMUSIC**.

Debe ser un reproductor de música local moderno, visualmente cuidado, sencillo de usar y fácil de empaquetar como Snap.

No debe ser un clon complejo de Spotify, Rhythmbox o Tauon. Debe centrarse en reproducir música local con buena interfaz, organización clara por carpetas y soporte de carátulas.

## Tecnología obligatoria

Usar:

- Rust
- GTK4
- Libadwaita
- GStreamer
- Cargo
- Snapcraft

Motivo técnico:

- GTK4 y Libadwaita encajan bien con aplicaciones modernas para GNOME/Ubuntu.
- gtk-rs es la vía oficial de bindings Rust para GTK.
- GStreamer tiene bindings Rust y es la base multimedia más adecuada en Linux.
- La aplicación debe funcionar correctamente en Wayland.
- El proyecto debe prepararse desde el inicio para empaquetado Snap.

Referencias técnicas de base:

- gtk-rs / GTK4 para Rust.
- Libadwaita para integración visual GNOME.
- gstreamer-rs para reproducción.
- Snapcraft para empaquetado.

## Nombre de la aplicación

Nombre visible:

AudioSimple

ID de aplicación:

org.fonteboa.AudioSimple

Nombre del binario:

audiosimple

## Principios de diseño

La aplicación debe ser:

- Sencilla.
- Rápida.
- Visualmente moderna.
- Estable.
- Sin cuentas de usuario.
- Sin servicios online.
- Sin telemetría.
- Sin publicidad.
- Sin sincronización en la nube.
- Sin base de datos compleja si no es necesaria.
- Sin depender de Electron.
- Sin depender de X11.
- Sin reproductor web embebido.

Debe funcionar principalmente con música almacenada en el equipo.

## Funcionalidades principales

### 1. Escaneo de música local

La aplicación debe permitir escanear el directorio de música del usuario.

Prioridad de rutas:

1. Directorio XDG Music del sistema.
2. `~/Música`
3. `~/Music`
4. Carpeta seleccionada manualmente por el usuario.

Debe haber un botón visible:

- “Escanear música”
- “Seleccionar carpeta”

El escaneo debe:

- Recorrer subdirectorios.
- Detectar archivos de audio.
- Mantener la estructura por carpetas.
- No bloquear la interfaz.
- Mostrar progreso o estado.
- Permitir reescanear.
- Ignorar archivos no reproducibles.
- Gestionar errores sin cerrar la aplicación.

Formatos mínimos:

- `.mp3`
- `.flac`
- `.ogg`
- `.opus`
- `.wav`
- `.m4a`
- `.aac`

La compatibilidad real dependerá de GStreamer y de los plugins instalados en el sistema o incluidos en el Snap.

### 2. Organización por carpetas

La vista principal debe organizar la música según la estructura real del directorio.

Ejemplo:

```text
Música/
  Extremoduro/
    Agila/
      01 - El día de la bestia.mp3
  Fito/
    Por la boca vive el pez/
      01 - Por la boca vive el pez.mp3
  Varios/
    Canciones sueltas/
      tema.mp3
````

La app debe mostrar algo similar a:

* Carpeta raíz
* Subcarpetas
* Álbumes o agrupaciones por carpeta
* Canciones dentro de cada carpeta

No depender exclusivamente de etiquetas ID3. Las etiquetas pueden usarse para mejorar el título mostrado, pero la organización principal debe ser por directorio.

### 3. Metadatos

Para cada pista, intentar obtener:

* Título
* Artista
* Álbum
* Duración
* Número de pista
* Ruta del archivo
* Formato
* Carátula si existe

Si no hay metadatos:

* Usar el nombre del archivo como título.
* Usar el nombre de la carpeta padre como álbum o grupo.
* No fallar.

### 4. Carátulas

La aplicación debe mostrar carátulas.

Orden de prioridad:

1. Imagen embebida en los metadatos del archivo.
2. Archivos de imagen en la misma carpeta:

   * `cover.jpg`
   * `cover.png`
   * `folder.jpg`
   * `folder.png`
   * `front.jpg`
   * `front.png`
   * `album.jpg`
   * `album.png`
3. Primera imagen razonable encontrada en la carpeta.
4. Imagen genérica generada por la aplicación.

Debe mostrar carátula en:

* Tarjeta de carpeta/álbum.
* Panel de reproducción actual.
* Vista de pista actual.

Debe cachear miniaturas para no regenerarlas todo el tiempo.

La caché debe guardarse en una ruta de datos de usuario apropiada, no dentro del directorio de música.

### 5. Reproducción

Usar GStreamer para reproducir audio.

Controles mínimos:

* Reproducir
* Pausar
* Detener
* Siguiente
* Anterior
* Barra de progreso
* Tiempo actual
* Duración total
* Volumen
* Silenciar

Comportamiento:

* Doble clic en una canción reproduce esa canción.
* Al terminar una canción, pasa automáticamente a la siguiente.
* Si se reproduce una carpeta, se genera cola con sus canciones.
* Si se reproduce una pista individual, se reproduce desde esa pista dentro de su carpeta o lista actual.
* Debe mostrarse claramente qué canción está sonando.

### 6. Cola de reproducción

Debe existir una cola sencilla.

Funcionalidades:

* Ver canciones pendientes.
* Reproducir desde la cola.
* Limpiar cola.
* Añadir carpeta a la cola.
* Añadir canción a la cola.
* Reordenar cola si es razonablemente sencillo.
* No hace falta implementar playlists complejas en la primera versión.

### 7. Interfaz gráfica

Usar GTK4 + Libadwaita.

La interfaz debe tener diseño moderno y limpio.

Estructura recomendada:

* Barra superior con:

  * Nombre de la app.
  * Botón de escaneo.
  * Botón de selección de carpeta.
  * Botón de configuración.
* Panel lateral izquierdo:

  * Carpetas detectadas.
  * Acceso rápido a “Toda la música”.
  * Acceso rápido a “Carpetas”.
  * Acceso rápido a “Cola”.
* Zona central:

  * Vista tipo tarjetas para carpetas/álbumes.
  * Vista de canciones al entrar en una carpeta.
* Barra inferior fija:

  * Carátula pequeña.
  * Canción actual.
  * Artista/álbum si existe.
  * Botones de reproducción.
  * Barra de progreso.
  * Volumen.

La app debe verse bien en:

* Pantalla grande.
* Ventana mediana.
* Tema claro.
* Tema oscuro.

Debe usar componentes Libadwaita cuando sea razonable:

* `AdwApplication`
* `AdwApplicationWindow`
* `HeaderBar`
* `ToastOverlay`
* `PreferencesWindow` si hay configuración
* widgets adaptativos si proceden

### 8. Búsqueda

Incluir búsqueda local sencilla.

Debe permitir buscar por:

* Nombre de archivo
* Título
* Artista
* Álbum
* Carpeta

La búsqueda debe ser rápida y no bloquear la interfaz.

### 9. Configuración

Guardar configuración básica:

* Última carpeta escaneada.
* Volumen.
* Tema preferido, si se implementa.
* Última canción reproducida, si es sencillo.
* Posición de ventana, si es sencillo.

Usar rutas estándar XDG.

No guardar configuración en rutas absolutas raras.

### 10. Base de datos o índice

Crear un índice local de la biblioteca.

Puede usarse SQLite o un archivo estructurado si el proyecto es pequeño.

Preferencia:

* SQLite mediante `rusqlite` o alternativa estable.

El índice debe guardar:

* Rutas de archivos.
* Carpetas.
* Metadatos básicos.
* Duración.
* Ruta de carátula cacheada.
* Fecha de modificación del archivo.
* Tamaño del archivo.

El reescaneo debe detectar:

* Archivos nuevos.
* Archivos eliminados.
* Archivos modificados.

### 11. Rendimiento

La app debe ser fluida.

Requisitos:

* No bloquear la UI durante escaneo.
* No cargar todas las carátulas enormes en memoria.
* Generar miniaturas.
* Usar carga diferida cuando sea posible.
* Evitar consumo excesivo de CPU.
* Evitar escaneos permanentes innecesarios.

### 12. Gestión de errores

La aplicación no debe cerrarse por errores comunes.

Debe manejar:

* Carpeta sin permisos.
* Archivo corrupto.
* Archivo no soportado.
* Falta de plugins GStreamer.
* Carátula inválida.
* Base de datos dañada.
* Ruta eliminada.
* Música en disco externo no conectado.

Mostrar errores con mensajes comprensibles.

Ejemplo:

“No se pudo reproducir este archivo. Puede faltar soporte para este formato de audio.”

### 13. Empaquetado Snap

Preparar el proyecto para generar Snap.

Debe incluir:

```text
snap/snapcraft.yaml
```

El Snap debe declarar los permisos necesarios:

* `home`
* `audio-playback`
* `desktop`
* `desktop-legacy` si hiciera falta
* `wayland`
* `x11` solo si es necesario como fallback
* `gsettings`
* `opengl` si GTK lo necesita
* `removable-media` opcional, documentado, para música en discos externos

No pedir permisos innecesarios.

Debe funcionar con confinamiento estricto si es posible.

Incluir en el proyecto:

* Archivo `.desktop`
* Icono SVG
* Metadatos básicos
* Instrucciones de construcción

### 14. Instalación en desarrollo

Incluir instrucciones para Ubuntu 26.04:

```bash
sudo apt install build-essential cargo rustc pkg-config libgtk-4-dev libadwaita-1-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
```

También indicar instalación de plugins GStreamer recomendados:

```bash
sudo apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly
```

No ejecutar comandos destructivos.

No modificar el sistema sin indicarlo claramente.

### 15. Estructura esperada del proyecto

Crear una estructura similar a:

```text
audiosimple/
  AGENT.md
  Cargo.toml
  README.md
  LICENSE
  src/
    main.rs
    app.rs
    ui/
      mod.rs
      window.rs
      player_bar.rs
      library_view.rs
      folder_view.rs
      queue_view.rs
      preferences.rs
    audio/
      mod.rs
      player.rs
      gst_backend.rs
    library/
      mod.rs
      scanner.rs
      metadata.rs
      cover.rs
      database.rs
      models.rs
    config/
      mod.rs
      settings.rs
    util/
      mod.rs
      paths.rs
      errors.rs
  data/
    org.fonteboa.AudioSimple.desktop
    org.fonteboa.AudioSimple.svg
    org.fonteboa.AudioSimple.metainfo.xml
  snap/
    snapcraft.yaml
```

### 16. Calidad del código

El código debe ser:

* Claro.
* Modular.
* Idiomático en Rust.
* Sin `unwrap()` en rutas críticas.
* Con gestión de errores mediante `Result`.
* Con comentarios donde ayuden.
* Sin sobreingeniería.
* Sin dependencias innecesarias.

Preferir crates mantenidas y ampliamente usadas.

### 17. Dependencias sugeridas

Evaluar estas dependencias, pero no añadirlas si no son necesarias:

* `gtk4`
* `libadwaita`
* `glib`
* `gio`
* `gstreamer`
* `gstreamer-audio`
* `gstreamer-pbutils`
* `rusqlite`
* `walkdir`
* `lofty` para metadatos de audio
* `image` para miniaturas
* `dirs` o `directories` para rutas XDG
* `anyhow` o `thiserror` para errores
* `serde`
* `serde_json`
* `tracing`

Antes de fijar versiones, comprobar compatibilidad actual.

### 18. Prioridades de implementación

Implementar por fases.

#### Fase 1

Crear ventana principal GTK4/Libadwaita.

Debe abrir correctamente en Ubuntu 26.04.

#### Fase 2

Implementar backend GStreamer.

Debe reproducir un archivo seleccionado manualmente.

#### Fase 3

Implementar escaneo de carpeta.

Debe listar archivos de audio por carpetas.

#### Fase 4

Implementar reproducción desde lista.

Doble clic en pista.

Siguiente/anterior.

#### Fase 5

Implementar carátulas.

Primero carátulas por archivo `cover.jpg/folder.jpg`.

Después carátulas embebidas.

#### Fase 6

Implementar índice persistente.

SQLite o sistema equivalente.

#### Fase 7

Mejorar interfaz visual.

Tarjetas, panel lateral, barra inferior, tema claro/oscuro.

#### Fase 8

Preparar Snap.

Crear `snapcraft.yaml`, `.desktop`, icono y README de empaquetado.

### 19. Criterios de aceptación

La aplicación se considera funcional cuando:

* Compila con `cargo build`.
* Se abre sin errores.
* Permite seleccionar una carpeta de música.
* Escanea subcarpetas.
* Muestra canciones organizadas por carpetas.
* Muestra carátulas cuando existen.
* Reproduce MP3 y FLAC con GStreamer.
* Permite pausar, continuar, avanzar y retroceder.
* Al acabar una canción pasa a la siguiente.
* No bloquea la interfaz durante el escaneo.
* Guarda la última carpeta usada.
* Incluye `snap/snapcraft.yaml`.
* Incluye README con instalación, ejecución y empaquetado.

### 20. Restricciones importantes

No usar:

* Electron.
* Node.js como base de la app.
* Servidor local HTTP.
* WebView como interfaz principal.
* Python.
* Qt.
* X11 obligatorio.
* Servicios online.
* Telemetría.
* Descarga automática de carátulas desde internet en la primera versión.

No implementar en la primera versión:

* Streaming.
* Letras de canciones.
* Ecualizador complejo.
* Sincronización móvil.
* Biblioteca musical en la nube.
* Login.
* Recomendaciones.
* Radio online.
* Podcasts.

### 21. Resultado esperado de Codex

Codex debe generar un proyecto completo, no solo fragmentos.

Debe entregar:

* Código fuente Rust.
* Interfaz GTK4/Libadwaita.
* Backend GStreamer funcional.
* Escáner de música local.
* Organización por carpetas.
* Soporte básico de carátulas.
* Configuración persistente.
* README claro.
* `snapcraft.yaml` inicial.
* Instrucciones para compilar y probar.

Antes de terminar, Codex debe revisar que el proyecto compile o, si no puede compilar en el entorno disponible, debe explicar exactamente qué dependencia falta y cómo instalarla.

