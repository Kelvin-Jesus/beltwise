# Beltwise — briefing de logo

## O jogo em uma frase

Beltwise é um jogo de automação para celular e navegador: você pousa num planeta congelado,
constrói esteiras e máquinas, e usa a fábrica para devolver a vida ao planeta
(gelo → água → musgo → floresta).

## O que o logo precisa comunicar

1. **Automação e fluxo:** uma esteira levando itens, com ordem e ritmo.
2. **Um planeta ganhando vida:** gelo virando verde. É um jogo otimista.
3. **Tom:** engenhoso, calmo e satisfatório. Não é jogo de guerra nem gacha brilhante.
   Referências de *tom* (não de forma): Mini Motorways, Shapez 2, Islanders, Builderment.

## Onde ele vai aparecer

- Ícone do app instalado (PWA), 512×512, e ícone *maskable*: o sistema recorta em círculo ou
  squircle, então o símbolo precisa caber nos 80% centrais.
- Favicon de 16 a 32 px. Precisa ser reconhecível pequeno.
- Tela de título do jogo (símbolo + nome) sobre fundo escuro.

## Diretrizes visuais

- Flat e vetorial, com formas geométricas grossas e no máximo 4 cores além do fundo.
- Silhueta forte: continua reconhecível em preto e branco e a 32 px.
- Sem texto dentro do símbolo. O nome vai separado.
- Sem 3D realista, brilho, bevel, textura, sombras longas ou linhas finas.

## Paleta (a mesma da interface)

| Uso | Cor |
| --- | --- |
| Fundo | `#12161C` grafite azulado |
| Indústria e itens | `#F59E0B` âmbar |
| Gelo | `#7DD3FC` azul-gelo |
| Vida | `#34D399` verde |
| Neutro claro | `#F1F5F9` |

## Conceitos (em ordem de preferência)

- **A. Planeta com anel-esteira:** um planeta pequeno, metade gelo e metade verde, com um anel
  inclinado que é uma esteira levando 2 ou 3 caixinhas âmbar.
- **B. "B" de esteira:** a letra B desenhada por um único caminho de esteira (com setas), e um
  brotinho verde saindo do topo.
- **C. Folha-esteira:** uma esteira em loop que forma a silhueta de uma folha, com itens âmbar
  circulando.

## Entregáveis ideais

1. Símbolo quadrado de 1024×1024 (SVG ou PNG) com fundo sólido `#12161C`.
2. O mesmo símbolo com fundo transparente.
3. Opcional: versão horizontal com símbolo + "Beltwise".

## Prompts

Os prompts estão em inglês porque os modelos de imagem respondem melhor assim.

### Conceito A (principal)

```
Minimal flat vector app icon logo for a cozy factory-automation game called "Beltwise". A small round planet, its left half frozen pale ice-blue and its right half fresh leaf-green, encircled by a tilted orbital ring that is a conveyor belt carrying three small amber cube crates. Bold, simple geometric shapes with thick silhouettes. Only 4 flat colors: ice blue #7DD3FC, leaf green #34D399, amber #F59E0B, off-white #F1F5F9, on a solid deep navy background #12161C. Centered with generous padding, rounded-square app icon, readable at 32 pixels. No text, no gradients, no 3D, no shadows, no fine details.
```

### Conceito B

```
Minimal flat vector monogram logo: a bold rounded letter "B" drawn by one continuous conveyor belt path, with small chevron arrows showing motion and two tiny amber cube crates riding on it. A small green two-leaf sprout grows from the top of the letter. Friendly, sturdy geometric strokes. Flat colors only: amber #F59E0B, ice blue #7DD3FC, leaf green #34D399, off-white #F1F5F9, on a solid deep navy background #12161C. Centered app icon, no other text, no gradients, no shading.
```

### Conceito C

```
Minimal flat vector logo symbol: a conveyor belt loop shaped like the outline of a single leaf, with small amber cube crates moving along it. The inside of the leaf is split into frozen ice blue at the bottom and fresh green at the top. Bold geometric shapes, only 4 flat colors (#F59E0B, #7DD3FC, #34D399, #F1F5F9) on a solid deep navy background #12161C. Centered app icon, readable at small sizes. No text, no gradients, no 3D.
```

### Versão com o nome

Use o Ideogram ou o ChatGPT, que escrevem texto bem.

```
Horizontal logo lockup on a solid deep navy background #12161C. On the left, a minimal flat icon: a small planet, half ice blue and half green, circled by a tilted conveyor-belt ring carrying amber crates. On the right, the word "Beltwise" in a bold, rounded, friendly geometric sans-serif, off-white #F1F5F9, where the dot of the "i" is a small amber square crate. Flat vector, 4 colors, no gradients, no tagline, no other text.
```

### Prompt negativo (para modelos que aceitam)

```
text, letters, words, watermark, signature, photorealistic, 3d render, glossy, bevel, emboss, gradient, drop shadow, texture, noise, thin lines, busy background, mockup, multiple icons, generic gear
```

Na versão com o nome, tire `text, letters, words` da lista.

## Dicas por ferramenta

- **Recraft** (recomendado, porque exporta SVG de verdade): estilo *Vector art* (flat), com
  as 4 cores definidas na paleta. Exporte em SVG.
- **Midjourney v7:** acrescente `--ar 1:1 --style raw --stylize 50 --no text, gradient, 3d, shadow`.
  Escolha a melhor das 4 e use *Vary (Subtle)*.
- **ChatGPT (imagem):** cole o prompt. Se vier detalhado demais, peça "simpler, fewer shapes,
  thicker lines".
- **Ideogram:** estilo *Design*, com o *Magic Prompt* desligado. É o melhor para a versão com o nome.
- **Flux ou SDXL:** use o prompt negativo e vetorize o resultado depois.

## Como avaliar

- Apertando os olhos, dá para ver "planeta + esteira" a 32 px?
- Funciona em preto e branco?
- Cabe num círculo (ícone maskable) sem cortar o anel?
- Combina com a interface escura do jogo?

## Depois

Salve o resultado em `design/logos/` (SVG ou PNG de 1024 px) e me avise. Eu gero o favicon, os
ícones do PWA (180, 192, 512 e maskable) e a tela de título a partir dele.
