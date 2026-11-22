<h1 align="center">
  <img src="apps/docs/app/latch-logo.png" width="360" alt="Latch">
</h1>
<p align="center"><strong>Feedback continuo para tu agente de programación.</strong></p>
<p align="center">Latch ejecuta los linters, las pruebas y otras comprobaciones de tu proyecto mientras el agente trabaja, y devuelve los fallos a la sesión.</p>
<p align="center">
  <a href="#instalar-un-plugin">Instalar un plugin</a> ·
  <a href="#ejecutar-tu-primera-comprobación">Primera comprobación</a> ·
  <a href="#cómo-funciona">Cómo funciona</a> ·
  <a href="#compatibilidad">Compatibilidad</a> ·
  <a href="#instalar-la-cli-independiente">CLI independiente</a> ·
  <a href="https://barakchamo.github.io/latch/">Documentación</a>
</p>
<p align="center">
  <a href="README.md">English</a> ·
  <a href="README_zh.md">简体中文</a> ·
  <a href="README_ja.md">日本語</a> ·
  <a href="README_ko.md">한국어</a> ·
  <a href="README_es.md">Español</a>
</p>

Cuando escribimos código, el IDE nos da feedback sobre la marcha. El resaltado de sintaxis facilita la lectura de los cambios, el linter y el formateador al guardar ayudan a detectar problemas pronto, y la comprobación de tipos revela si un cambio rompe código en otro sitio. Latch les da ese mismo feedback a los agentes de programación.

- **Ejecuta las comprobaciones automáticamente.** Los hooks del entorno las ponen en marcha después de una edición compatible o al terminar un turno que modificó el árbol de trabajo. El agente no necesita recordar qué ejecutar ni cuándo.
- **Dedica el contexto a los problemas que requieren atención.** Por defecto, las comprobaciones que pasan no generan salida. Si alguna falla, Latch muestra el diagnóstico que el agente necesita para actuar.

## Cómo funciona

Con Latch puedes detectar un error del linter justo después de editar un archivo o enterarte de que una prueba falló cuando termina un turno. Reutiliza los comandos que ya ejecutas en local o en CI y guárdalos en el proyecto para que todo el equipo pueda usar las mismas comprobaciones con distintos agentes.

Los plugins conectan los hooks nativos de cada entorno con dos eventos comunes de Latch. Los eventos disponibles dependen del entorno. Los comandos se quedan en el proyecto y funcionan con todas las integraciones.

| Evento | Cuándo se ejecutan las comprobaciones |
| --- | --- |
| `after_edit` | Después de que una herramienta de edición compatible modifique el árbol de trabajo. Ideal para comprobaciones rápidas con un linter. |
| `turn_end` | Cuando el agente termina un turno que modificó el árbol de trabajo. También detecta cambios hechos desde la shell y es un buen momento para ejecutar pruebas. |

- **Omite el trabajo sin cambios.** Latch compara los archivos visibles para Git antes y después de la edición o el turno. Por defecto, las ediciones que dejan el contenido igual y los turnos de solo lectura no activan las comprobaciones, aunque ya haya cambios sin confirmar en el árbol de trabajo.
- **Mantén las comprobaciones exitosas en silencio.** Por defecto, el éxito no genera salida. Así los fallos se ven más fácilmente en la sesión.
- **Ejecuta las comprobaciones que correspondan al cambio.** Filtra por ruta o extensión. Si solo cambia un archivo Markdown, puedes ejecutar su linter y omitir las comprobaciones de TypeScript.
- **Mantén breves los informes de fallos.** La salida compacta conserva líneas útiles, como los errores y las aserciones. El agente puede consultar el registro guardado para obtener más detalles sin repetir la comprobación.

## Instalar un plugin

Instala Latch en tu agente en macOS o Linux con glibc (ARM64 y x64). En el primer uso, el plugin descarga y verifica el binario de Latch para tu plataforma. Después de instalarlo, abre una sesión nueva en tu proyecto Git.

### Codex

```sh
codex plugin marketplace add BarakChamo/latch
codex plugin add latch@latch-tools
```

Si Codex indica que ya agregaste `latch-tools` desde otra fuente, ejecuta `codex plugin marketplace remove latch-tools` y repite los dos comandos anteriores.

En una sesión nueva de Codex, elige **Review hooks** cuando aparezca el aviso o abre `/hooks`. Aprueba ahí los hooks de Latch.

### Claude Code

```sh
claude plugin marketplace add BarakChamo/latch
claude plugin install latch@latch-tools
```

Abre una sesión nueva de Claude Code en tu proyecto.

### GitHub Copilot CLI

```sh
copilot plugin marketplace add BarakChamo/latch
copilot plugin install latch@latch-tools
```

### Pi

```sh
pi install git:github.com/BarakChamo/latch
```

Usa Pi 1.5.2 o posterior.

### OpenCode

Consulta la versión con `opencode --version`. A partir de la v2.3.2, ejecuta este comando.

```sh
opencode plugin add 'github:BarakChamo/latch'
```

Si usas OpenCode v1 y vas a instalar Latch por primera vez, actualiza a v2. [Detalles de OpenCode](https://barakchamo.github.io/latch/docs/plugins/#opencode)

### Grok Build

```sh
grok plugin install BarakChamo/latch --trust
```

Verificado con Grok Build 1.0.92.

### Kimi Code

Ejecuta este comando en Kimi Code.

```text
/plugins install https://github.com/BarakChamo/latch/tree/main
```

Elige **Trust and install** y después ejecuta `/reload` o inicia una sesión nueva.

### OpenHands

En Agent Canvas, abre **Customize → Plugins → Add plugin**. Introduce `github:BarakChamo/latch` como fuente y pulsa **Install**. Una vez habilitado, el plugin se carga en las conversaciones locales nuevas.
[Configuración de OpenHands SDK y Canvas](https://barakchamo.github.io/latch/docs/plugins/#openhands)

Para OpenHands CLI, [instala el binario independiente de Latch](https://barakchamo.github.io/latch/docs/cli/#install-the-cli) y ejecuta `latch install openhands` en tu proyecto Git.

[Administrar plugins](https://barakchamo.github.io/latch/docs/plugins/)

## Ejecutar tu primera comprobación

**Latch puede ejecutar comandos de proyectos en cualquier lenguaje.** Puedes prescindir de `latch.json` si defines `hook:turnend` en `package.json` o `hook-turnend` como script de consola en un proyecto uv empaquetado. Para cualquier otro comando, añádelo a `latch.json`.
[Nombres y detección automática de scripts](docs/usage.md#package-script-discovery)

### Configurar comprobaciones

Por ejemplo, si `make lint` ya funciona en tu proyecto, crea este `latch.json` en el directorio donde ejecutas el comando.

```json
{
  "hooks": {
    "turn_end": {
      "commands": "make lint"
    }
  }
}
```

Usa una cadena para un solo comando o listas anidadas de argumentos para varios comandos. [Sintaxis de los comandos](docs/usage.md#command-syntax)

Puedes crear el archivo a mano o ejecutar `latch config init` con la [CLI independiente](#instalar-la-cli-independiente). Puedes combinar comandos explícitos con los scripts detectados automáticamente.

Inicia el agente en tu proyecto y pídele que haga una edición que provoque un fallo. Si tu entorno entrega el informe en el siguiente mensaje, espera a que termine el turno y envía un mensaje en la misma sesión. Pídele al agente que llame a `latch_status` para comprobar si Latch ejecutó las comprobaciones o las omitió. Deshaz el cambio de prueba. Las comprobaciones que pasan no generan salida.
[Guía de inicio rápido](https://barakchamo.github.io/latch/docs/quickstart/#run-your-first-check) · [Ejemplos ejecutables](examples/)

### Filtros y salida de fallos

Este ejemplo ejecuta tu script `npm test` al final del turno cuando cambia un archivo `.ts` o `.tsx` en `src/`. El mismo archivo debe cumplir ambos filtros.

```json
{
  "hooks": {
    "turn_end": {
      "paths": ["src/**"],
      "extensions": ["ts", "tsx"],
      "commands": [["npm", "test"]],
      "failure_output": "compact"
    }
  }
}
```

Este es un fragmento del informe compacto de una ejecución de Vitest con fallos.

```text
Latch checks failed.
latch: command 1 ("npm") failed: exit status: 1
stdout:
Tests  1 failed | 36 passed (37)
[output compacted from 3175 bytes]
stderr:
FAIL  tests/message.test.ts > greets the caller by name
AssertionError: expected 'Hi, Latch!' to be 'Hello, Latch!'
Saved log: …/r-kgMl3D6EnjseUc7Nal1Wgf8Z.log
```

Abre el registro guardado para leer la salida completa sin volver a ejecutar las pruebas. Usa `tail` para ver las últimas líneas de cada flujo de salida, o deja `failure_output` sin definir para ver el extracto predeterminado.

[Ejemplo ejecutable de filtros](examples/filters/) · [Modos de salida de fallos](https://barakchamo.github.io/latch/docs/configuration/#failure-output-modes)

### Darle al agente una oportunidad de actuar ante un fallo

Con Latch v0.1.3 o posterior, establece `"turn_end_feedback": "continue_once"` en el nivel superior de `latch.json`. En los entornos compatibles, el agente recibe automáticamente el informe de fallo al final del turno y tiene una oportunidad de responder. No hace falta que envíes otro mensaje y Latch no repite la comprobación automáticamente. En Grok Build, esta opción requiere Latch v0.1.4 o posterior.

[Agentes compatibles y cómo reciben los fallos](docs/usage.md#act-on-a-turn-end-failure)

## Instalar la CLI independiente

[Descarga Latch para tu plataforma](https://barakchamo.github.io/latch/docs/cli/#install-the-cli). Descomprime el archivo y coloca `latch` en un directorio de tu `PATH`. Ejecuta `latch --version` para confirmar la instalación.

## Compatibilidad

Las columnas locales indican cuándo recibe el agente los fallos por defecto, salvo donde se indica que hay que activar la opción.

| Entorno | Después de editar | Al final del turno | Nube gestionada |
| --- | --- | --- | --- |
| Codex | Mismo turno | Siguiente mensaje | CLI de Latch o CI |
| Claude Code | Mismo turno | Siguiente mensaje | Verificado con un solo repositorio |
| GitHub Copilot CLI | Mismo turno | Siguiente mensaje | Hooks del repositorio · sin verificar |
| OpenCode v1 y v2 | Mismo turno | Siguiente mensaje | Sin verificar |
| Pi | Mismo turno | Siguiente solicitud al modelo | Sin verificar |
| OpenHands CLI y SDK | Siguiente mensaje | Siguiente mensaje | Hooks del espacio de trabajo · sin verificar |
| OpenHands Agent Canvas | — | Verificado al activar la opción | Sin verificar |
| Kimi Code CLI | — | Siguiente mensaje | Sin verificar |
| Grok Build 1.0.92 | Mismo turno | El modelo no recibe el fallo | Sin verificar |

«Siguiente mensaje» significa el próximo mensaje que envíes en la misma sesión y proyecto. Kimi 2.7.2 admite `continue_once` en sesiones interactivas y ACP. Usa `report` con `kimi -p`.

Con Latch 0.1.6 o posterior, ejecuta `latch install cloud claude`, `latch install cloud copilot` o `latch install cloud openhands` en tu copia local del repositorio y haz commit del archivo que actualice Latch. Se conserva la configuración existente. En Claude Code en la nube, el agente recibe de inmediato los fallos de las comprobaciones posteriores a una edición. Los fallos de fin de turno llegan con el siguiente mensaje por defecto o de forma automática con `continue_once`.

[Detalles de los hooks y el feedback](https://barakchamo.github.io/latch/docs/hooks/) · [Configuración para la nube](https://barakchamo.github.io/latch/docs/cloud/)

## Documentación

- [Ciclo de vida de los hooks y dónde aparecen los fallos](docs/usage.md#hooks-and-feedback)
- [Detección de scripts del proyecto](https://barakchamo.github.io/latch/docs/project-scripts/) y [ejemplos ejecutables](examples/)
- [Configuración, filtros y esquema JSON](docs/usage.md#configuration)
- [Comandos de la CLI, herramientas del plugin y registros guardados](docs/usage.md#commands-and-saved-logs)
- [Guías de integración con agentes](plugins/), [configuración para la nube](docs/usage.md#hosted-sessions) y [solución de problemas](docs/usage.md#troubleshooting)

## Licencia

Latch se distribuye bajo la [Licencia Apache 2.0](LICENSE).
