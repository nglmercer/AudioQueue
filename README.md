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

## Instalación Rápida

```bash
# Clonar el repositorio
git clone <repository-url>
cd AudioQueue

# Compilar
cargo build --release

# El binario estará en target/release/audioqueue
```

## Uso Básico

```bash
# Añadir archivo a la cola
./target/release/audioqueue add /ruta/musica.mp3

# Ver la cola
./target/release/audioqueue list

# Iniciar reproducción
./target/release/audioqueue play

# Pausar
./target/release/audioqueue pause

# Siguiente pista
./target/release/audioqueue next
```

## 📚 Documentación

### Guías de Usuario
- [Guía de Instalación](docs/installation.md) - Instalación detallada y prerrequisitos
- [Guía de Uso](docs/usage.md) - Todos los comandos y opciones disponibles
- [Ejemplos Prácticos](docs/examples.md) - Flujos de trabajo y casos de uso
- [Integración](docs/integration.md) - Uso desde otros lenguajes de programación

### Documentación Técnica
- [Arquitectura](docs/architecture.md) - Estructura interna y componentes
- [Formatos Soportados](docs/formats.md) - Lista de formatos de audio compatibles
- [Configuración](docs/configuration.md) - Variables de entorno y archivos de config
- [API Reference](docs/api-reference.md) - Referencia de la API interna

### Desarrollo
- [Guía de Desarrollo](docs/development.md) - Cómo contribuir y desarrollar
- [Testing](docs/testing.md) - Guía completa de testing
- [Troubleshooting](docs/troubleshooting.md) - Problemas comunes y soluciones
- [Roadmap](docs/roadmap.md) - Plan de desarrollo futuro

## 🚀 Quick Start

```bash
# 1. Crear una playlist básica
audioqueue add cancion1.mp3
audioqueue add cancion2.mp3
audioqueue add cancion3.flac

# 2. Ver la cola
audioqueue list

# 3. Reproducir
audioqueue play

# 4. Controlar la reproducción
audioqueue pause  # Pausar
audioqueue resume # Reanudar
audioqueue next   # Siguiente
audioqueue volume 0.8  # Ajustar volumen
```

## 🏗️ Arquitectura

AudioQueue está construido con una arquitectura modular:

```
CLI Layer (main.rs)
    ↓
AudioQueueManager
    ↓
┌─────────────────┬─────────────────┐
│   AudioQueue    │  AudioEmitter   │
│   (Cola)        │  (Reproducción) │
└─────────────────┴─────────────────┘
    ↓                 ↓
Symphonia         Rodio
(Metadatos)      (Audio Output)
```

## 🧪 Testing

```bash
# Ejecutar todos los tests
cargo test

# Tests específicos
cargo test --test basic_tests
cargo test --test integration_playback_tests

# Setup de archivos de prueba
cargo run --bin setup_test_audio
```

## 📋 Formatos Soportados

AudioQueue soporta los siguientes formatos a través de Symphonia:
- **MP3** - MPEG Audio Layer III
- **FLAC** - Free Lossless Audio Codec  
- **OGG** - Ogg Vorbis
- **WAV** - Waveform Audio File Format
- **M4A/AAC** - MPEG-4 Audio
- Y muchos más...

Ver [Formatos Soportados](docs/formats.md) para la lista completa.

## 🤝 Contribuir

¡Las contribuciones son bienvenidas! Por favor lee la [Guía de Desarrollo](docs/development.md) para más información.

## 📄 Licencia

Este proyecto está licenciado bajo la MIT License - ver el archivo [LICENSE](LICENSE) para detalles.

---

**¿Necesitas ayuda?** Revisa la [documentación](docs/) o abre un [issue](../../issues).