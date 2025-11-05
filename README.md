# AudioQueue

Un gestor de cola de audio en Rust con interfaz de línea de comandos, similar a ffmpeg, construido con symphonia para el procesamiento de audio.

## Características

- **Gestión de cola**: Añadir, eliminar, reordenar pistas de audio
- **Reproducción**: Play, pause, resume, next, previous
- **Navegación**: Saltar a posiciones específicas en la cola
- **Metadatos**: Extracción automática de información de archivos (título, artista, duración)
- **Soporte de formatos**: Amplia compatibilidad mediante symphonia (MP3, FLAC, OGG, WAV, etc.)
- **Control de volumen**: Ajuste de volumen de reproducción
- **Estado en tiempo real**: Visualización del estado actual de reproducción y cola

## Instalación

### Prerrequisitos

- Rust 1.70+ 
- Sistema con capacidad de audio (ALSA en Linux, WASAPI en Windows, CoreAudio en macOS)

### Compilación

```bash
git clone <repository-url>
cd AudioQueue
cargo build --release
```

El binario compilado estará en `target/release/audioqueue`.

## Uso

### Comandos Básicos

#### Añadir archivos a la cola

```bash
# Añadir al final de la cola
audioqueue add /path/to/music.mp3

# Añadir en posición específica
audioqueue add /path/to/music.mp3 --position 2

# Usar rutas relativas
audioqueue add ./music/flac/file.flac
```

#### Ver la cola

```bash
# Listar todas las pistas en la cola
audioqueue list
```

#### Control de reproducción

```bash
# Iniciar reproducción
audioqueue play

# Pausar
audioqueue pause

# Reanudar
audioqueue resume

# Siguiente pista
audioqueue next

# Pista anterior
audioqueue previous

# Saltar a posición específica
audioqueue jump 3
```

#### Gestión de la cola

```bash
# Eliminar pista en posición 2
audioqueue remove 2

# Mover pista de posición 1 a 3
audioqueue move 1 3

# Limpiar toda la cola
audioqueue clear
```

#### Estado y control

```bash
# Mostrar estado actual
audioqueue status

# Ajustar volumen (0.0 a 1.0)
audioqueue volume 0.7
```

#### Iniciar como servicio/daemon

```bash
# Iniciar el gestor en modo daemon
audioqueue start
```

## Ejemplos de Flujo de Trabajo

### Ejemplo 1: Crear una playlist y reproducir

```bash
# Añadir varios archivos
audioqueue add ~/Music/rock/song1.mp3
audioqueue add ~/Music/jazz/song2.flac --position 0
audioqueue add ~/Music/electronic/song3.ogg

# Ver la cola
audioqueue list

# Iniciar reproducción
audioqueue play

# Ver estado durante reproducción
audioqueue status
```

### Ejemplo 2: Gestión dinámica durante reproducción

```bash
# Suponiendo que ya hay música reproduciéndose

# Añadir nueva canción al final
audioqueue add ~/Downloads/new_song.mp3

# Mover canción actual al principio
audioqueue move 5 0

# Saltar a la nueva canción
audioqueue jump 0

# Ajustar volumen
audioqueue volume 0.5
```

## Uso desde otros lenguajes (vía binario)

Puedes integrar AudioQueue desde cualquier lenguaje invocando el binario `audioqueue` como un subproceso. Esto es ideal para:

- Ejecutar comandos atómicos (add, list, play, pause, resume, next, previous, jump, clear, status, volume).
- Obtener el estado mediante `audioqueue status` y parsear su salida estándar.

Recomendaciones:

- Mantén el binario accesible en PATH o usa la ruta absoluta a `target/release/audioqueue` (en Windows `audioqueue.exe`).
- Para “escuchar” el estado, realiza polling periódico con `audioqueue status` y parsea stdout.
- Si necesitas un proceso residente, puedes ejecutar `audioqueue start` y seguir emitiendo comandos como procesos separados. Actualmente no hay un canal IPC estable, por lo que el patrón soportado es invocación por proceso y polling de `status`.

### Ejemplo: Node.js

Control y polling de estado usando `child_process`:

```js
// control.js
const { execFile } = require('node:child_process');
const path = require('node:path');

// Ajusta esta ruta si no tienes el binario en PATH
const BIN = process.platform === 'win32'
  ? path.resolve(__dirname, 'target/release/audioqueue.exe')
  : path.resolve(__dirname, 'target/release/audioqueue');

function run(cmd, args = []) {
  return new Promise((resolve, reject) => {
    execFile(BIN, [cmd, ...args], { windowsHide: true }, (err, stdout, stderr) => {
      if (err) return reject(new Error(stderr || err.message));
      resolve(stdout.toString());
    });
  });
}

(async () => {
  // Añadir y reproducir
  await run('add', ['test_data/SoundHelix-Song-1.mp3']);
  await run('play');

  // Polling de estado cada 2s
  setInterval(async () => {
    try {
      const out = await run('status');
      // Parseo simple por líneas; puedes usar regex para extraer campos
      console.log('[status]', out.trim());
    } catch (e) {
      console.error('status error:', e.message);
    }
  }, 2000);

  // Ejemplos de control
  setTimeout(() => run('volume', ['0.5']).catch(console.error), 3000);
  setTimeout(() => run('pause').catch(console.error), 6000);
  setTimeout(() => run('resume').catch(console.error), 9000);
})();
```

### Ejemplo: Python

```python
# control.py
import subprocess
import sys
from pathlib import Path

BIN = Path('target/release/audioqueue' + ('.exe' if sys.platform == 'win32' else ''))

def run(cmd, *args):
    res = subprocess.run([str(BIN), cmd, *map(str, args)], capture_output=True, text=True)
    if res.returncode != 0:
        raise RuntimeError(res.stderr)
    return res.stdout

print(run('add', 'test_data/SoundHelix-Song-1.mp3'))
print(run('play'))
print(run('status'))
```

## Ejecutables de ejemplo (examples/)

Para reducir el peso del paquete, los utilitarios de prueba se movieron a `examples/`:

- **setup_test_audio**: prepara `test_data/` con archivos de prueba.
- **test_audioqueue**: smoke tests básicos sobre el binario.

Cómo ejecutarlos:

```bash
cargo run --release --example setup_test_audio
cargo run --release --example test_audioqueue
```

## Testing y Desarrollo

### Archivos de Audio para Testing

Para probar completamente AudioQueue, se recomienda tener archivos de audio de diferentes formatos y características. Aquí están las fuentes recomendadas:

#### Archivos de Prueba Recomendados

**1. Archivos de Prueba Universales (Múltiples Formatod)**

```bash
# Crear directorio de testing
mkdir -p test_audio

# Descargar archivos de prueba de diferentes formatos
wget -O test_audio/sample.mp3 "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3"
wget -O test_audio/sample.flac "https://filesamples.com/formats/flac/sample1.flac"
wget -O test_audio/sample.ogg "https://filesamples.com/formats/ogg/sample1.ogg"
wget -O test_audio/sample.wav "https://filesamples.com/formats/wav/sample1.wav"
wget -O test_audio/sample.m4a "https://filesamples.com/formats/m4a/sample1.m4a"
```

**2. Archivos de Prueba Específicos**

```bash
# Archivos con diferentes características de audio
curl -L -o test_audio/short_test.mp3 "https://freewavesamples.com/files/Yamaha-V50-Rock-Kit-1.mp3"
curl -L -o test_audio/stereo_test.flac "https://www2.cs.uic.edu/~i101/SoundFiles/BabyElephantWalk60.wav"
curl -L -o test_audio/mono_test.wav "https://www2.cs.uic.edu/~i101/SoundFiles/PinkPanther30.wav"

# Archivos con metadatos complejos
curl -L -o test_audio/metadata_test.mp3 "https://chinmay-.github.io/audio-testing-files/metadata.mp3"
```

**3. Archivos de Prueba de Calidad Profesional**

```bash
# Archivos de alta calidad para pruebas avanzadas
curl -L -o test_audio/hq_test.flac "https://www.dropbox.com/s/k5w9h2y7x6d4x0h/96khz24bit.flac?dl=1"
curl -L -o test_audio/surround_test.ogg "https://www2.cs.uic.edu/~i101/SoundFiles/IMissionImpossible60.wav"
```

#### Fuentes Confiables para Archivos de Prueba

**Repositorios Oficiales:**
- [SoundHelix](https://www.soundhelix.com/) - Música generada para testing
- [FileSamples](https://filesamples.com/) - Archivos de prueba en múltiples formatos
- [Free Wave Samples](https://freewavesamples.com/) - Archivos de audio gratuitos
- [UIC Sound Files](https://www2.cs.uic.edu/~i101/SoundFiles/) - Archivos académicos de testing

**Generadores de Audio:**
- [Online Audio Converter](https://online-audio-converter.com/) - Para convertir entre formatos
- [Audacity](https://www.audacityteam.org/) - Para generar tonos de prueba

#### Script de Configuración para Testing

```bash
#!/bin/bash
# setup_test_audio.sh

echo "Configurando archivos de audio para testing..."

# Crear estructura de directorios
mkdir -p test_audio/{short,medium,long,various_formats}

# Descargar archivos de diferentes duraciones
echo "Descargando archivos de prueba..."
wget -q -O test_audio/short/5sec.mp3 "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3" 
# Cortar a 5 segundos (requiere ffmpeg)

# Archivos de formatos variados
echo "Descargando formatos variados..."
formats=("mp3" "flac" "ogg" "wav" "m4a")
for format in "${formats[@]}"; do
    echo "  Descargando .$format..."
    curl -L -o "test_audio/various_formats/test.$format" \
         "https://filesamples.com/formats/$format/sample1.$format"
done

# Crear archivo de playlist de prueba
cat > test_audio/test_playlist.m3u << EOF
# Playlist de prueba para AudioQueue
test_audio/various_formats/test.mp3
test_audio/various_formats/test.flac
test_audio/various_formats/test.ogg
test_audio/various_formats/test.wav
test_audio/various_formats/test.m4a
EOF

echo "¡Configuración completada! Archivos en ./test_audio/"
```

#### Pruebas Automatizadas

**Prueba Básica de Funcionalidad:**

```bash
#!/bin/bash
# test_basic_functionality.sh

echo "=== Pruebas Básicas de AudioQueue ==="

# Compilar proyecto
echo "Compilando..."
cargo build --release

# Prueba de validación de archivos
echo "Validando archivos de prueba..."
./target/release/audioqueue validate test_audio/various_formats/test.mp3
./target/release/audioqueue validate test_audio/various_formats/test.flac
./target/release/audioqueue validate test_audio/various_formats/test.ogg

# Prueba de adición a cola
echo "Añadiendo archivos a la cola..."
./target/release/audioqueue add test_audio/various_formats/test.mp3
./target/release/audioqueue add test_audio/various_formats/test.flac
./target/release/audioqueue add test_audio/various_formats/test.ogg

# Mostrar cola
echo "Cola actual:"
./target/release/audioqueue list

# Prueba de metadatos
echo "Extrayendo metadatos..."
./target/release/audioqueue metadata test_audio/various_formats/test.mp3

echo "Pruebas básicas completadas."
```

#### Casos de Testing Específicos

**1. Testing de Formatos:**
- MP3: Compatibilidad con diferentes bitrates (128, 192, 320 kbps)
- FLAC: Audio sin pérdida, diferentes frecuencias de muestreo
- OGG: Codec Vorbis, diferentes calidades
- WAV: PCM, diferentes profundidades de bits (16, 24, 32)
- M4A: AAC, diferentes perfiles

**2. Testing de Metadatos:**
- Archivos con artwork/cover art
- Tags ID3v1, ID3v2.3, ID3v2.4
- Metadatos Vorbis Comments
- Archivos sin metadatos
- Metadatos con caracteres especiales (Unicode)

**3. Testing de Edge Cases:**
- Archivos muy cortos (< 1 segundo)
- Archivos muy largos (> 1 hora)
- Archivos con diferentes frecuencias de muestreo
- Audio mono vs stereo
- Archivos corruptos o dañados

**4. Testing de Rendimiento:**
- Colas con muchos archivos (100+ tracks)
- Cambios rápidos de track
- Uso de memoria con archivos grandes

#### Integración con Tests de Rust

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_supported_formats() {
        let formats = vec!["test.mp3", "test.flac", "test.ogg", "test.wav", "test.m4a"];
        
        for format in formats {
            let path = PathBuf::from(format!("test_audio/various_formats/{}", format));
            if path.exists() {
                assert!(AudioQueue::validate_audio_file(&path).unwrap_or(false), 
                       "Formato {} debería ser válido", format);
            }
        }
    }

    #[test]
    fn test_metadata_extraction() {
        let path = PathBuf::from("test_audio/various_formats/test.mp3");
        if path.exists() {
            let track = AudioQueue::extract_metadata(&path).unwrap();
            assert!(track.title.is_some() || track.path.file_name().is_some());
        }
    }
}
```

## Arquitectura

### Componentes Principales

1. **AudioQueue**: Gestiona la cola de pistas, metadatos y estado de reproducción
2. **AudioEmitter**: Maneja la reproducción de audio usando rodio + symphonia
3. **CLI Interface**: Procesa comandos del usuario usando clap

### Flujo de Arquitectura

```
CLI Commands → AudioQueueManager → AudioQueue (cola) + AudioEmitter (reproducción)
                      ↓
               Metadatos con symphonia
                      ↓
               Reproducción con rodio
```

### Emisor Interno vs Externo

**Emisor Interno (Implementación actual)**:
- Ventajas: Integración completa, control directo del estado
- Desventajas: Proceso único, si el proceso termina se detiene todo

**Emisor Externo (Posible extensión)**:
- Ventajas: Procesos separados, mayor robustez
- Desventajas: Comunicación IPC más compleja, sincronización

## Formatos Soportados

Symphonia soporta una amplia gama de formatos de audio:

- **Contenedores**: MP3, FLAC, OGG, WAV, M4A, AAC, WMA
- **Codecs**: MP3, FLAC, Vorbis, PCM, AAC, AC3, DTS

## Configuración

### Variables de Entorno

- `AUDIOQUEUE_DEVICE`: Dispositivo de audio de salida (opcional)
- `AUDIOQUEUE_VOLUME`: Volumen por defecto (0.0-1.0)

### Archivos de Configuración

En futuras versiones se añadirá soporte para archivos de configuración JSON/TOML.

## Troubleshooting

### Problemas Comunes

1. **"File does not exist"**: Verifica que la ruta sea correcta y el archivo exista
2. **"File is not a valid audio file"**: El formato no es compatible o el archivo está corrupto
3. **"No audio sink available"**: Problema con el sistema de audio del SO
4. **"Permission denied"**: Permisos insuficientes para leer el archivo o acceder al dispositivo de audio

### Depuración

Ejecuta con variable de entorno para logs detallados:
```bash
RUST_LOG=debug audioqueue list
```

## Desarrollo

### Estructura del Proyecto

```
src/
├── main.rs           # CLI y orquestación principal
├── audio_queue.rs    # Gestión de cola y metadatos
├── audio_emitter.rs  # Reproducción de audio
└── (módulos futuros)

Cargo.toml            # Dependencias
README.md            # Esta documentación
```

### Compilación para Desarrollo

```bash
cargo build
cargo test
cargo run -- add test.mp3
```

### Contribuciones

1. Fork del proyecto
2. Crear feature branch
3. Hacer commits con mensajes descriptivos
4. Abrir Pull Request

## Licencia

MIT License - ver archivo LICENSE para detalles.

## Testing y Desarrollo

### Archivos de Audio para Testing

Para probar completamente AudioQueue, se recomienda tener archivos de audio de diferentes formatos y características. Aquí están las fuentes recomendadas:

#### Archivos de Prueba Recomendados

**1. Archivos de Prueba Universales (Múltiples Formatod)**

```bash
# Crear directorio de testing
mkdir -p test_audio

# Descargar archivos de prueba de diferentes formatos
wget -O test_audio/sample.mp3 "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3"
wget -O test_audio/sample.flac "https://filesamples.com/formats/flac/sample1.flac"
wget -O test_audio/sample.ogg "https://filesamples.com/formats/ogg/sample1.ogg"
wget -O test_audio/sample.wav "https://filesamples.com/formats/wav/sample1.wav"
wget -O test_audio/sample.m4a "https://filesamples.com/formats/m4a/sample1.m4a"
```

**2. Archivos de Prueba Específicos**

```bash
# Archivos con diferentes características de audio
curl -L -o test_audio/short_test.mp3 "https://freewavesamples.com/files/Yamaha-V50-Rock-Kit-1.mp3"
curl -L -o test_audio/stereo_test.flac "https://www2.cs.uic.edu/~i101/SoundFiles/BabyElephantWalk60.wav"
curl -L -o test_audio/mono_test.wav "https://www2.cs.uic.edu/~i101/SoundFiles/PinkPanther30.wav"

# Archivos con metadatos complejos
curl -L -o test_audio/metadata_test.mp3 "https://chinmay-.github.io/audio-testing-files/metadata.mp3"
```

**3. Archivos de Prueba de Calidad Profesional**

```bash
# Archivos de alta calidad para pruebas avanzadas
curl -L -o test_audio/hq_test.flac "https://www.dropbox.com/s/k5w9h2y7x6d4x0h/96khz24bit.flac?dl=1"
curl -L -o test_audio/surround_test.ogg "https://www2.cs.uic.edu/~i101/SoundFiles/IMissionImpossible60.wav"
```

#### Fuentes Confiables para Archivos de Prueba

**Repositorios Oficiales:**
- [SoundHelix](https://www.soundhelix.com/) - Música generada para testing
- [FileSamples](https://filesamples.com/) - Archivos de prueba en múltiples formatos
- [Free Wave Samples](https://freewavesamples.com/) - Archivos de audio gratuitos
- [UIC Sound Files](https://www2.cs.uic.edu/~i101/SoundFiles/) - Archivos académicos de testing

**Generadores de Audio:**
- [Online Audio Converter](https://online-audio-converter.com/) - Para convertir entre formatos
- [Audacity](https://www.audacityteam.org/) - Para generar tonos de prueba

#### Script de Configuración para Testing

```bash
#!/bin/bash
# setup_test_audio.sh

echo "Configurando archivos de audio para testing..."

# Crear estructura de directorios
mkdir -p test_audio/{short,medium,long,various_formats}

# Descargar archivos de diferentes duraciones
echo "Descargando archivos de prueba..."
wget -q -O test_audio/short/5sec.mp3 "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3" 
# Cortar a 5 segundos (requiere ffmpeg)

# Archivos de formatos variados
echo "Descargando formatos variados..."
formats=("mp3" "flac" "ogg" "wav" "m4a")
for format in "${formats[@]}"; do
    echo "  Descargando .$format..."
    curl -L -o "test_audio/various_formats/test.$format" \
         "https://filesamples.com/formats/$format/sample1.$format"
done

# Crear archivo de playlist de prueba
cat > test_audio/test_playlist.m3u << EOF
# Playlist de prueba para AudioQueue
test_audio/various_formats/test.mp3
test_audio/various_formats/test.flac
test_audio/various_formats/test.ogg
test_audio/various_formats/test.wav
test_audio/various_formats/test.m4a
EOF

echo "¡Configuración completada! Archivos en ./test_audio/"
```

#### Pruebas Automatizadas

**Prueba Básica de Funcionalidad:**

```bash
#!/bin/bash
# test_basic_functionality.sh

echo "=== Pruebas Básicas de AudioQueue ==="

# Compilar proyecto
echo "Compilando..."
cargo build --release

# Prueba de validación de archivos
echo "Validando archivos de prueba..."
./target/release/audioqueue validate test_audio/various_formats/test.mp3
./target/release/audioqueue validate test_audio/various_formats/test.flac
./target/release/audioqueue validate test_audio/various_formats/test.ogg

# Prueba de adición a cola
echo "Añadiendo archivos a la cola..."
./target/release/audioqueue add test_audio/various_formats/test.mp3
./target/release/audioqueue add test_audio/various_formats/test.flac
./target/release/audioqueue add test_audio/various_formats/test.ogg

# Mostrar cola
echo "Cola actual:"
./target/release/audioqueue list

# Prueba de metadatos
echo "Extrayendo metadatos..."
./target/release/audioqueue metadata test_audio/various_formats/test.mp3

echo "Pruebas básicas completadas."
```

#### Casos de Testing Específicos

**1. Testing de Formatos:**
- MP3: Compatibilidad con diferentes bitrates (128, 192, 320 kbps)
- FLAC: Audio sin pérdida, diferentes frecuencias de muestreo
- OGG: Codec Vorbis, diferentes calidades
- WAV: PCM, diferentes profundidades de bits (16, 24, 32)
- M4A: AAC, diferentes perfiles

**2. Testing de Metadatos:**
- Archivos con artwork/cover art
- Tags ID3v1, ID3v2.3, ID3v2.4
- Metadatos Vorbis Comments
- Archivos sin metadatos
- Metadatos con caracteres especiales (Unicode)

**3. Testing de Edge Cases:**
- Archivos muy cortos (< 1 segundo)
- Archivos muy largos (> 1 hora)
- Archivos con diferentes frecuencias de muestreo
- Audio mono vs stereo
- Archivos corruptos o dañados

**4. Testing de Rendimiento:**
- Colas con muchos archivos (100+ tracks)
- Cambios rápidos de track
- Uso de memoria con archivos grandes

#### Integración con Tests de Rust

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_supported_formats() {
        let formats = vec!["test.mp3", "test.flac", "test.ogg", "test.wav", "test.m4a"];
        
        for format in formats {
            let path = PathBuf::from(format!("test_audio/various_formats/{}", format));
            if path.exists() {
                assert!(AudioQueue::validate_audio_file(&path).unwrap_or(false), 
                       "Formato {} debería ser válido", format);
            }
        }
    }

    #[test]
    fn test_metadata_extraction() {
        let path = PathBuf::from("test_audio/various_formats/test.mp3");
        if path.exists() {
            let track = AudioQueue::extract_metadata(&path).unwrap();
            assert!(track.title.is_some() || track.path.file_name().is_some());
        }
    }
}
```

## Roadmap Futuro

- [ ] Soporte para playlists (M3U, PLS)
- [ ] Comandos de shuffle/repeat
- [ ] Salida a múltiples dispositivos
- [ ] Integración con streaming
- [ ] GUI opcional
- [ ] Plugins de efectos
- [ ] Persistencia de cola entre sesiones
- [ ] Suite de tests automatizados completa
- [ ] Generador de archivos de prueba integrado