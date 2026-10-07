"""Molang fields built only of `query.has_biome_tag`, `||`, `&&`, `!`, `?:`, parentheses and numbers,
evaluated per overworld biome at transcription (as `Expr::ByBiome`)."""
import re

# Biome id -> tag set; features_to_rust.py fills it before transcribing.
TAGS = {}
_TOKEN = re.compile(r"\s*(query\.has_biome_tag\('([^']*)'\)|\|\||&&|[?:!()]|-?\d+(?:\.\d+)?)", re.I)


def _tokens(text):
    out, i = [], 0
    while i < len(text.strip()):
        m = _TOKEN.match(text, i)
        if not m:
            raise ValueError(f'unsupported Molang at {text[i:]!r}')
        out.append(('tag', m[2]) if m[2] is not None else ('op', m[1]))
        i = m.end()
    return out


def _evaluate(tokens, tags):
    pos = 0

    def peek():
        return tokens[pos][1] if pos < len(tokens) else None

    def take():
        nonlocal pos
        pos += 1
        return tokens[pos - 1]

    def primary():
        kind, value = take()
        if kind == 'tag':
            return 1.0 if value in tags else 0.0
        if value == '(':
            result = ternary()
            take()
            return result
        if value == '!':
            return 0.0 if primary() else 1.0
        return float(value)

    def conjunction():
        result = primary()
        while peek() == '&&':
            take()
            right = primary()
            result = 1.0 if result and right else 0.0
        return result

    def disjunction():
        result = conjunction()
        while peek() == '||':
            take()
            right = conjunction()
            result = 1.0 if result or right else 0.0
        return result

    def ternary():
        condition = disjunction()
        if peek() != '?':
            return condition
        take()
        yes = ternary()
        take()
        no = ternary()
        return yes if condition else no

    return ternary()


def by_biome(text):
    tokens = _tokens(text)
    pairs = [(biome, _evaluate(tokens, tags)) for biome, tags in sorted(TAGS.items())]
    return 'Expr::ByBiome(&[' + ', '.join(f'({b}, {v!r})' for b, v in pairs if v) + '])'
