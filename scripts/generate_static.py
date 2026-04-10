#!/usr/bin/env python3
"""Generate realistic static files for HttpArena benchmarks."""

import gzip
import hashlib
import json
import os
import random
import struct

try:
    import brotli
    HAS_BROTLI = True
except ImportError:
    HAS_BROTLI = False

OUT = os.path.join(os.path.dirname(__file__), '..', 'data', 'static')
random.seed(42)

# --- Vocabulary for realistic code generation ---

CSS_PROPS = [
    'display', 'position', 'top', 'right', 'bottom', 'left', 'float', 'clear',
    'margin', 'margin-top', 'margin-right', 'margin-bottom', 'margin-left',
    'padding', 'padding-top', 'padding-right', 'padding-bottom', 'padding-left',
    'width', 'height', 'min-width', 'min-height', 'max-width', 'max-height',
    'overflow', 'overflow-x', 'overflow-y', 'font-family', 'font-size',
    'font-weight', 'font-style', 'line-height', 'letter-spacing', 'text-align',
    'text-decoration', 'text-transform', 'color', 'background', 'background-color',
    'background-image', 'background-size', 'background-position', 'border',
    'border-top', 'border-right', 'border-bottom', 'border-left', 'border-radius',
    'box-shadow', 'opacity', 'z-index', 'cursor', 'transition', 'transform',
    'animation', 'flex', 'flex-direction', 'flex-wrap', 'justify-content',
    'align-items', 'align-self', 'gap', 'grid-template-columns', 'grid-template-rows',
    'grid-gap', 'white-space', 'word-break', 'visibility', 'outline', 'box-sizing',
    'vertical-align', 'content', 'list-style', 'pointer-events', 'user-select',
    'will-change', 'backface-visibility', 'perspective', 'filter', 'backdrop-filter',
    'clip-path', 'object-fit', 'object-position', 'scroll-behavior', 'touch-action',
    'appearance', '-webkit-appearance', 'text-overflow', 'resize',
]

CSS_VALUES = {
    'display': ['block', 'inline', 'inline-block', 'flex', 'inline-flex', 'grid', 'none', 'contents', 'table', 'table-cell'],
    'position': ['relative', 'absolute', 'fixed', 'sticky', 'static'],
    'overflow': ['hidden', 'auto', 'scroll', 'visible', 'clip'],
    'text-align': ['left', 'center', 'right', 'justify'],
    'font-weight': ['400', '500', '600', '700', '800', 'normal', 'bold'],
    'cursor': ['pointer', 'default', 'not-allowed', 'grab', 'text', 'move', 'crosshair'],
    'flex-direction': ['row', 'column', 'row-reverse', 'column-reverse'],
    'justify-content': ['flex-start', 'flex-end', 'center', 'space-between', 'space-around', 'space-evenly'],
    'align-items': ['flex-start', 'flex-end', 'center', 'stretch', 'baseline'],
    'text-transform': ['uppercase', 'lowercase', 'capitalize', 'none'],
    'white-space': ['nowrap', 'pre', 'pre-wrap', 'normal', 'break-spaces'],
    'box-sizing': ['border-box', 'content-box'],
    'visibility': ['visible', 'hidden', 'collapse'],
    'object-fit': ['cover', 'contain', 'fill', 'none', 'scale-down'],
}

COLORS = [
    '#1a1a2e', '#16213e', '#0f3460', '#e94560', '#533483', '#2b2d42', '#8d99ae',
    '#edf2f4', '#ef233c', '#d90429', '#f8f9fa', '#dee2e6', '#adb5bd', '#6c757d',
    '#495057', '#343a40', '#212529', '#0d6efd', '#6610f2', '#6f42c1', '#d63384',
    '#dc3545', '#fd7e14', '#ffc107', '#198754', '#20c997', '#0dcaf0', '#fff',
    '#000', 'transparent', 'inherit', 'currentColor', 'rgba(0,0,0,.1)',
    'rgba(0,0,0,.15)', 'rgba(0,0,0,.25)', 'rgba(0,0,0,.5)', 'rgba(0,0,0,.75)',
    'rgba(255,255,255,.1)', 'rgba(255,255,255,.5)', 'rgba(255,255,255,.85)',
    'hsla(220,13%,18%,.95)', 'hsla(0,0%,100%,.08)', 'var(--primary)', 'var(--secondary)',
    'var(--accent)', 'var(--bg)', 'var(--fg)', 'var(--border-color)', 'var(--muted)',
    'var(--surface)', 'var(--surface-hover)', 'var(--danger)', 'var(--success)',
    'var(--warning)', 'var(--info)',
]

UNITS = ['px', 'rem', 'em', '%', 'vh', 'vw', 'ch', 'vmin', 'vmax']
FONT_FAMILIES = [
    'system-ui,-apple-system,sans-serif', '"Inter",sans-serif', '"Roboto",sans-serif',
    '"SF Pro Display",sans-serif', 'monospace', '"JetBrains Mono",monospace',
    '"Segoe UI","Helvetica Neue",Arial,sans-serif', 'var(--font-sans)', 'var(--font-mono)',
    '"Georgia",serif', 'var(--font-heading)',
]
EASING = ['ease', 'ease-in', 'ease-out', 'ease-in-out', 'linear',
          'cubic-bezier(.4,0,.2,1)', 'cubic-bezier(.4,0,1,1)', 'cubic-bezier(0,0,.2,1)',
          'cubic-bezier(.68,-.55,.27,1.55)', 'cubic-bezier(.22,1,.36,1)']
MEDIA_QUERIES = [
    '@media(max-width:576px)', '@media(min-width:576px)', '@media(min-width:768px)',
    '@media(min-width:992px)', '@media(min-width:1200px)', '@media(min-width:1400px)',
    '@media(prefers-color-scheme:dark)', '@media(prefers-reduced-motion:reduce)',
    '@media print', '@media(hover:hover)',
]

COMPONENT_PREFIXES = [
    'btn', 'card', 'modal', 'nav', 'sidebar', 'header', 'footer', 'hero',
    'alert', 'badge', 'toast', 'tooltip', 'popover', 'dropdown', 'tab',
    'accordion', 'breadcrumb', 'pagination', 'progress', 'spinner', 'avatar',
    'chip', 'tag', 'table', 'form', 'input', 'select', 'checkbox', 'radio',
    'switch', 'slider', 'calendar', 'datepicker', 'dialog', 'drawer', 'menu',
    'toolbar', 'stepper', 'timeline', 'carousel', 'gallery', 'grid', 'list',
    'divider', 'skeleton', 'placeholder', 'banner', 'callout', 'panel',
    'widget', 'search', 'filter', 'sort', 'upload', 'preview',
]

COMPONENT_SUFFIXES = [
    '', '-sm', '-md', '-lg', '-xl', '-primary', '-secondary', '-outline',
    '-ghost', '-link', '-icon', '-text', '-header', '-body', '-footer',
    '-title', '-subtitle', '-content', '-wrapper', '-container', '-inner',
    '-overlay', '-backdrop', '-trigger', '-close', '-toggle', '-label',
    '-group', '-item', '-active', '-disabled', '-loading', '-error',
    '-success', '-warning', '-info', '-danger', '-dark', '-light',
    '-rounded', '-flat', '-raised', '-floating', '-sticky', '-fixed',
    '-responsive', '-compact', '-dense', '-expanded', '-collapsed',
]

JS_VAR_PREFIXES = [
    'handle', 'render', 'update', 'create', 'delete', 'fetch', 'parse',
    'validate', 'format', 'transform', 'compute', 'calculate', 'process',
    'init', 'setup', 'configure', 'register', 'subscribe', 'dispatch',
    'emit', 'on', 'get', 'set', 'is', 'has', 'should', 'can', 'will',
    'resolve', 'reject', 'merge', 'clone', 'serialize', 'deserialize',
    'encode', 'decode', 'compress', 'decompress', 'encrypt', 'decrypt',
    'normalize', 'sanitize', 'escape', 'unescape', 'throttle', 'debounce',
    'memoize', 'cache', 'invalidate', 'refresh', 'reload', 'retry',
    'defer', 'delay', 'schedule', 'cancel', 'abort', 'reset', 'clear',
    'filter', 'sort', 'group', 'map', 'reduce', 'find', 'search', 'match',
    'replace', 'split', 'join', 'trim', 'pad', 'truncate', 'wrap',
    'bind', 'apply', 'call', 'invoke', 'execute', 'run', 'start', 'stop',
]

JS_NOUNS = [
    'User', 'Item', 'List', 'Data', 'Config', 'State', 'Props', 'Context',
    'Event', 'Error', 'Result', 'Response', 'Request', 'Query', 'Mutation',
    'Action', 'Reducer', 'Store', 'Cache', 'Buffer', 'Stream', 'Channel',
    'Socket', 'Connection', 'Session', 'Token', 'Auth', 'Permission',
    'Role', 'Profile', 'Account', 'Settings', 'Preferences', 'Theme',
    'Layout', 'Component', 'Element', 'Node', 'Tree', 'Graph', 'Map',
    'Set', 'Queue', 'Stack', 'Heap', 'Pool', 'Registry', 'Factory',
    'Builder', 'Observer', 'Listener', 'Handler', 'Middleware', 'Plugin',
    'Module', 'Package', 'Bundle', 'Chunk', 'Slice', 'Fragment', 'Ref',
    'Hook', 'Effect', 'Memo', 'Callback', 'Promise', 'Timer', 'Interval',
    'Timeout', 'Animation', 'Transition', 'Route', 'Path', 'Params',
    'Schema', 'Model', 'Entity', 'Record', 'Field', 'Column', 'Row',
    'Page', 'View', 'Screen', 'Panel', 'Tab', 'Modal', 'Dialog',
    'Form', 'Input', 'Option', 'Value', 'Label', 'Message', 'Notification',
    'Alert', 'Warning', 'Info', 'Debug', 'Log', 'Metric', 'Counter',
    'Gauge', 'Histogram', 'Summary', 'Trace', 'Span', 'Tag', 'Annotation',
]

# SVG icon paths (realistic icon path data)
SVG_ICON_NAMES = [
    'home', 'search', 'user', 'settings', 'bell', 'mail', 'heart', 'star',
    'bookmark', 'share', 'download', 'upload', 'edit', 'delete', 'copy',
    'paste', 'undo', 'redo', 'refresh', 'sync', 'filter', 'sort', 'grid',
    'list', 'menu', 'close', 'check', 'plus', 'minus', 'arrow-up',
    'arrow-down', 'arrow-left', 'arrow-right', 'chevron-up', 'chevron-down',
    'chevron-left', 'chevron-right', 'external-link', 'link', 'unlink',
    'lock', 'unlock', 'eye', 'eye-off', 'calendar', 'clock', 'map-pin',
    'phone', 'camera', 'image', 'video', 'music', 'file', 'folder',
    'archive', 'trash', 'flag', 'tag', 'hash', 'at-sign', 'globe',
    'wifi', 'bluetooth', 'battery', 'cpu', 'monitor', 'smartphone',
    'tablet', 'printer', 'server', 'database', 'cloud', 'sun', 'moon',
    'thermometer', 'droplet', 'wind', 'zap', 'activity', 'trending-up',
    'bar-chart', 'pie-chart', 'layers', 'layout', 'sidebar', 'columns',
    'maximize', 'minimize', 'move', 'crop', 'scissors', 'tool', 'code',
    'terminal', 'git-branch', 'git-commit', 'git-merge', 'git-pull-request',
    'package', 'box', 'shield', 'award', 'target', 'crosshair', 'compass',
    'navigation', 'map', 'anchor', 'life-buoy', 'aperture', 'disc',
    'headphones', 'mic', 'volume', 'speaker', 'radio', 'cast', 'airplay',
    'send', 'inbox', 'message-circle', 'message-square', 'users', 'user-plus',
]


def css_value(prop):
    """Generate a realistic CSS value for a given property."""
    if prop in CSS_VALUES:
        return random.choice(CSS_VALUES[prop])
    if 'color' in prop or prop == 'background' and random.random() < 0.5:
        return random.choice(COLORS)
    if 'font-family' in prop:
        return random.choice(FONT_FAMILIES)
    if 'shadow' in prop:
        x, y, b, s = [random.choice([0,1,2,4,8,12,16,24]) for _ in range(4)]
        return f'{x}px {y}px {b}px {s}px rgba(0,0,0,.{random.randint(5,30)})'
    if 'transition' in prop:
        p = random.choice(['all','opacity','transform','color','background','border','box-shadow'])
        d = random.choice(['.15s','.2s','.25s','.3s','.35s','.4s','150ms','200ms','250ms','300ms'])
        return f'{p} {d} {random.choice(EASING)}'
    if 'transform' in prop:
        t = random.choice(['translateX','translateY','translate','scale','rotate','skewX','skewY'])
        if t == 'rotate': return f'rotate({random.randint(-180,180)}deg)'
        if t == 'scale': return f'scale({random.choice([".95",".98","1","1.02","1.05","1.1"])})'
        v = random.choice(['-100%','-50%','-8px','-4px','0','4px','8px','50%','100%'])
        return f'{t}({v})'
    if 'radius' in prop:
        v = random.choice([0,2,3,4,6,8,12,16,24,9999])
        return f'{v}px' if v != 9999 else '9999px'
    if 'z-index' in prop:
        return str(random.choice([1,2,5,10,50,100,999,1000,9999,10000]))
    if prop == 'opacity':
        return str(random.choice([0,.05,.1,.15,.2,.25,.3,.4,.5,.6,.7,.75,.8,.85,.9,.95,1]))
    if any(x in prop for x in ['margin','padding','top','right','bottom','left','gap','width','height']):
        if random.random() < 0.15:
            return 'auto' if 'margin' in prop else '100%'
        v = random.choice([0,1,2,4,6,8,10,12,14,16,20,24,28,32,36,40,48,56,64,72,80,96])
        u = random.choice(['px','rem']) if v > 0 else '0'
        if u == 'rem': return f'{v/16:.4g}rem'
        return f'{v}px' if v else '0'
    if 'font-size' in prop:
        v = random.choice([10,11,12,13,14,15,16,18,20,22,24,28,32,36,40,48,56,64])
        return f'{v}px'
    if 'line-height' in prop:
        return str(random.choice([1,1.15,1.25,1.35,1.4,1.5,1.6,1.75,2]))
    if 'letter-spacing' in prop:
        return random.choice(['-.05em','-.025em','0','0.025em','.05em','.1em','.15em'])
    if 'border' in prop and 'radius' not in prop:
        w = random.choice([1,2,3])
        s = random.choice(['solid','dashed','dotted','none'])
        return f'{w}px {s} {random.choice(COLORS)}'
    if 'animation' in prop:
        name = random.choice(['fadeIn','fadeOut','slideIn','slideOut','pulse','bounce','spin','shake'])
        d = random.choice(['.2s','.3s','.5s','.8s','1s','1.5s','2s'])
        return f'{name} {d} {random.choice(EASING)}'
    if 'filter' in prop or 'backdrop' in prop:
        f = random.choice(['blur','brightness','contrast','saturate','grayscale','sepia'])
        if f == 'blur': return f'blur({random.choice([1,2,4,8,12,16,20,24])}px)'
        return f'{f}({random.choice([".5",".75",".9","1","1.1","1.25","1.5","2"])})'
    return random.choice(COLORS)


def gen_css_rule():
    """Generate a single CSS rule with 2-6 declarations."""
    prefix = random.choice(COMPONENT_PREFIXES)
    suffix = random.choice(COMPONENT_SUFFIXES)
    pseudo = random.choice(['', ':hover', ':focus', ':active', ':first-child', ':last-child',
                           '::before', '::after', ':focus-visible', ':not(:last-child)',
                           ':nth-child(2n)', ':disabled', '[aria-expanded="true"]', ':checked',
                           ':placeholder-shown', '::placeholder', ':focus-within', ':empty'])
    combinator = random.choice(['', ' ', ' > ', ' + ', ' ~ '])
    child = ''
    if combinator.strip():
        child = random.choice([f'.{random.choice(COMPONENT_PREFIXES)}{random.choice(COMPONENT_SUFFIXES)}',
                               'span', 'div', 'a', 'p', 'svg', 'img', 'input', 'button', 'label',
                               '[role="button"]', '[data-active]', '*'])
    selector = f'.{prefix}{suffix}{combinator}{child}{pseudo}'
    props = random.sample(CSS_PROPS, random.randint(2, 6))
    decls = ';'.join(f'{p}:{css_value(p)}' for p in props)
    return f'{selector}{{{decls}}}'


def gen_css_keyframe():
    """Generate a CSS keyframe animation."""
    name = random.choice(['fadeIn','fadeOut','slideInUp','slideInDown','slideInLeft','slideInRight',
                          'pulse','bounce','spin','shake','grow','shrink','blink','swing','wobble',
                          'flash','rubberBand','jello','heartBeat','flipInX','flipInY','zoomIn','zoomOut',
                          'rollIn','lightSpeedIn','rotateIn','jackInTheBox','hinge','backInUp'])
    steps = random.choice([
        ['from', 'to'],
        ['0%', '50%', '100%'],
        ['0%', '25%', '50%', '75%', '100%'],
    ])
    rules = []
    for step in steps:
        props = random.sample(['opacity', 'transform', 'visibility', 'filter', 'color'], random.randint(1, 3))
        decls = ';'.join(f'{p}:{css_value(p)}' for p in props)
        rules.append(f'{step}{{{decls}}}')
    return f'@keyframes {name}{{{"".join(rules)}}}'


def gen_css_custom_props():
    """Generate CSS custom properties block."""
    props = [
        ('--primary', random.choice(COLORS)),
        ('--secondary', random.choice(COLORS)),
        ('--accent', random.choice(COLORS)),
        ('--bg', random.choice(['#fff','#fafafa','#f5f5f5','#1a1a2e','#0f0f1a'])),
        ('--fg', random.choice(['#212529','#1a1a2e','#e8e8e8','#f5f5f5'])),
        ('--border-color', random.choice(['#dee2e6','#e5e7eb','rgba(0,0,0,.1)','rgba(255,255,255,.1)'])),
        ('--radius-sm', random.choice(['2px','3px','4px'])),
        ('--radius-md', random.choice(['6px','8px'])),
        ('--radius-lg', random.choice(['12px','16px'])),
        ('--shadow-sm', f'0 1px 2px rgba(0,0,0,.{random.randint(5,15)})'),
        ('--shadow-md', f'0 4px 6px -1px rgba(0,0,0,.{random.randint(8,20)})'),
        ('--shadow-lg', f'0 10px 15px -3px rgba(0,0,0,.{random.randint(10,25)})'),
        ('--font-sans', random.choice(FONT_FAMILIES)),
        ('--font-mono', '"JetBrains Mono",monospace'),
        ('--font-size-xs', '.75rem'),
        ('--font-size-sm', '.875rem'),
        ('--font-size-md', '1rem'),
        ('--font-size-lg', '1.125rem'),
        ('--font-size-xl', '1.25rem'),
        ('--transition-fast', '.15s ease'),
        ('--transition-normal', '.25s ease'),
        ('--transition-slow', '.4s ease'),
        ('--z-dropdown', '1000'),
        ('--z-sticky', '1020'),
        ('--z-fixed', '1030'),
        ('--z-modal-backdrop', '1040'),
        ('--z-modal', '1050'),
        ('--z-popover', '1060'),
        ('--z-tooltip', '1070'),
    ]
    decls = ';'.join(f'{k}:{v}' for k, v in props)
    return f':root{{{decls}}}'


def generate_css(target_kb, include_vars=False, include_keyframes=False, include_media=False):
    """Generate CSS content targeting approximately target_kb kilobytes."""
    parts = []
    if include_vars:
        parts.append(gen_css_custom_props())
    target_bytes = target_kb * 1024
    while len(''.join(parts)) < target_bytes:
        r = random.random()
        if include_keyframes and r < 0.05:
            parts.append(gen_css_keyframe())
        elif include_media and r < 0.15:
            # Media query block with 2-5 rules
            mq = random.choice(MEDIA_QUERIES)
            inner = ''.join(gen_css_rule() for _ in range(random.randint(2, 5)))
            parts.append(f'{mq}{{{inner}}}')
        else:
            parts.append(gen_css_rule())
    return ''.join(parts)[:target_bytes]


# --- JavaScript generation ---

def gen_js_string():
    """Generate a realistic JS string literal."""
    templates = [
        lambda: f'"{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}"',
        lambda: f'"/{"/".join(random.choice(["api","v1","v2","auth","users","items","data","config","search","admin"]) for _ in range(random.randint(1,3)))}"',
        lambda: f'"application/{random.choice(["json","xml","octet-stream","x-www-form-urlencoded","pdf"])}"',
        lambda: f'"{random.choice(["GET","POST","PUT","PATCH","DELETE","HEAD","OPTIONS"])}"',
        lambda: f'"data-{random.choice(["id","type","state","index","key","value","action","target","source","ref"])}"',
        lambda: f'"aria-{random.choice(["label","hidden","expanded","selected","disabled","live","role","controls","describedby","labelledby"])}"',
        lambda: f'`{random.choice(["Error","Warning","Info","Debug","Trace"])}: ${{{random.choice(JS_VAR_PREFIXES)}}}`',
        lambda: f'"{random.choice(["click","submit","change","input","focus","blur","keydown","keyup","resize","scroll","load","error","mouseover","mouseout","touchstart","touchend","pointerdown","pointerup","dragstart","drop"])}"',
    ]
    return random.choice(templates)()


def gen_js_function(depth=0):
    """Generate a realistic JS function."""
    name = f'{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}'
    params = ','.join(random.sample(['e','t','n','r','o','i','a','s','c','l','u','d','p','f','h','m','g','v','b','y','w','x','k','_','$'],
                                    random.randint(0, 4)))
    body_parts = []
    num_statements = random.randint(3, 12)
    for _ in range(num_statements):
        body_parts.append(gen_js_statement(depth))
    body = ';'.join(body_parts)
    style = random.choice(['function', 'const', 'export'])
    if style == 'function':
        return f'function {name}({params}){{{body}}}'
    elif style == 'const':
        return f'const {name}=({params})=>{{{body}}}'
    else:
        return f'export function {name}({params}){{{body}}}'


def gen_js_statement(depth=0):
    """Generate a single JS statement."""
    r = random.random()
    v1 = random.choice('etnoiasc')
    v2 = random.choice('lduphfmg')

    if r < 0.15:
        # Variable declaration
        val = random.choice([
            gen_js_string(),
            str(random.randint(0, 9999)),
            random.choice(['true', 'false', 'null', 'void 0', '""', '[]', '{}', 'new Map', 'new Set', 'Object.create(null)']),
            f'{v2}.{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}',
            f'{v2}[{gen_js_string()}]',
            f'document.querySelector({gen_js_string()})',
        ])
        return f'const {v1}={val}'
    elif r < 0.25:
        # If statement
        cond = random.choice([
            f'{v1}', f'!{v1}', f'{v1}==null', f'{v1}!==null', f'typeof {v1}==="undefined"',
            f'{v1}.length>0', f'{v1}&&{v1}.{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}',
            f'Array.isArray({v1})', f'{v1} instanceof Error', f'{v1}>{random.randint(0,100)}',
        ])
        then = gen_js_statement(depth+1)
        if random.random() < 0.3 and depth < 2:
            el = gen_js_statement(depth+1)
            return f'if({cond}){{{then}}}else{{{el}}}'
        return f'if({cond}){{{then}}}'
    elif r < 0.32:
        # Try/catch
        try_body = gen_js_statement(depth+1)
        return f'try{{{try_body}}}catch({v2}){{console.error({v2})}}'
    elif r < 0.4:
        # Method call chain
        obj = random.choice([v1, v2, 'this', 'self', 'window', 'document', 'globalThis'])
        methods = []
        for _ in range(random.randint(1, 4)):
            m = f'{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}'
            args = ','.join(random.choice([gen_js_string(), str(random.randint(0,99)), 'true', 'null', v1, v2])
                          for _ in range(random.randint(0, 2)))
            methods.append(f'.{m}({args})')
        return f'{obj}{"".join(methods)}'
    elif r < 0.48:
        # Array operation
        op = random.choice(['map', 'filter', 'reduce', 'forEach', 'find', 'some', 'every', 'flatMap', 'sort'])
        if op == 'reduce':
            return f'{v1}.{op}(({v2},n)=>{v2}+n,0)'
        elif op == 'sort':
            return f'{v1}.{op}((a,b)=>a-b)'
        else:
            return f'{v1}.{op}({v2}=>{v2}.{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)})'
    elif r < 0.55:
        # Object destructuring/spread
        keys = random.sample([f'{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}' for _ in range(8)], random.randint(2, 5))
        return f'const {{{",".join(keys)}}}={v1}'
    elif r < 0.62:
        # Promise/async
        return random.choice([
            f'await {random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}({v1})',
            'return new Promise((resolve,reject)=>{setTimeout(()=>resolve(' + v1 + '),' + str(random.choice([0,10,50,100,500])) + ')})',
            f'{v1}.then({v2}=>{v2}).catch(err=>console.error(err))',
        ])
    elif r < 0.68:
        # DOM operation
        return random.choice([
            f'{v1}.addEventListener({gen_js_string()},{v2})',
            f'{v1}.classList.{random.choice(["add","remove","toggle"])}({gen_js_string()})',
            f'{v1}.setAttribute({gen_js_string()},{gen_js_string()})',
            f'{v1}.style.{random.choice(CSS_PROPS[:20])}={gen_js_string()}',
            f'{v1}.innerHTML={gen_js_string()}',
            f'{v1}.textContent={gen_js_string()}',
            f'{v1}.appendChild(document.createElement({gen_js_string()}))',
        ])
    elif r < 0.74:
        # Return
        return random.choice([
            f'return {v1}',
            f'return{{{",".join(f"{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}:{random.choice([v1,v2,"null","true",str(random.randint(0,99))])}" for _ in range(random.randint(2,5)))}}}',
            f'return {v1}?{v2}:null',
        ])
    elif r < 0.80:
        # Switch
        cases = ''.join(f'case {gen_js_string()}:{gen_js_statement(depth+1)};break;' for _ in range(random.randint(2, 4)))
        return f'switch({v1}){{{cases}default:break}}'
    elif r < 0.86:
        # For loop
        return f'for(let {v2}=0;{v2}<{v1}.length;{v2}++){{{gen_js_statement(depth+1)}}}'
    elif r < 0.92:
        # Ternary assignment
        return f'const {v2}={v1}?{gen_js_string()}:{gen_js_string()}'
    else:
        # Console/logging
        level = random.choice(['log','warn','error','debug','info','trace'])
        return f'console.{level}({gen_js_string()},{v1})'


def gen_js_class():
    """Generate a JS class."""
    name = random.choice(JS_NOUNS)
    parent = random.choice(['', f' extends {random.choice(JS_NOUNS)}'])
    methods = []
    # Constructor
    params = ','.join(random.sample([f'{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}' for _ in range(6)], random.randint(1, 4)))
    ctor_body = ';'.join(f'this.{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}={random.choice("etnoiasc")}' for _ in range(random.randint(2, 5)))
    methods.append(f'constructor({params}){{{ctor_body}}}')
    # Regular methods
    for _ in range(random.randint(3, 8)):
        mname = f'{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}'
        mparams = ','.join(random.sample(list('etnoiasclud'), random.randint(0, 3)))
        mbody = ';'.join(gen_js_statement(0) for _ in range(random.randint(2, 6)))
        prefix = random.choice(['', 'async ', 'static ', 'get '])
        methods.append(f'{prefix}{mname}({mparams}){{{mbody}}}')
    return f'class {name}{parent}{{{"".join(methods)}}}'


def gen_js_object_literal():
    """Generate a large object literal (like a config or constants object)."""
    name = f'{random.choice(["DEFAULT","INITIAL","BASE","APP","CONFIG","CONSTANTS","DEFAULTS","OPTIONS","SETTINGS"])}_{random.choice(JS_NOUNS).upper()}'
    entries = []
    for _ in range(random.randint(8, 20)):
        key = f'{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}'
        val = random.choice([
            gen_js_string(),
            str(random.randint(0, 9999)),
            random.choice(['true', 'false', 'null']),
            f'[{",".join(gen_js_string() for _ in range(random.randint(2,5)))}]',
            f'{{{",".join(f"{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}:{gen_js_string()}" for _ in range(random.randint(2,4)))}}}',
        ])
        entries.append(f'{key}:{val}')
    return f'const {name}={{{",".join(entries)}}}'


def generate_js(target_kb):
    """Generate JS content targeting approximately target_kb kilobytes."""
    parts = []
    # Imports at top
    for _ in range(random.randint(5, 15)):
        names = ','.join(random.sample([f'{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}' for _ in range(8)], random.randint(1, 4)))
        module = random.choice([
            f'./{random.choice(["utils","helpers","api","config","constants","types","hooks","store","services","components","lib"])}',
            f'@app/{random.choice(["core","ui","data","auth","router","state","i18n","theme","logger","analytics"])}',
        ])
        parts.append(f'import{{{names}}}from"{module}"')
    target_bytes = target_kb * 1024
    while len(';'.join(parts)) < target_bytes:
        r = random.random()
        if r < 0.35:
            parts.append(gen_js_function())
        elif r < 0.5:
            parts.append(gen_js_class())
        elif r < 0.65:
            parts.append(gen_js_object_literal())
        else:
            # IIFE or module pattern
            body = ';'.join(gen_js_statement(0) for _ in range(random.randint(5, 15)))
            parts.append(f'(function(){{{body}}})()')
    return ';'.join(parts)[:target_bytes]


# --- HTML generation ---

HTML_TAGS = ['div', 'span', 'p', 'a', 'button', 'img', 'input', 'label', 'section',
             'article', 'aside', 'nav', 'main', 'header', 'footer', 'ul', 'li',
             'h1', 'h2', 'h3', 'h4', 'form', 'select', 'option', 'textarea',
             'table', 'thead', 'tbody', 'tr', 'th', 'td', 'figure', 'figcaption',
             'details', 'summary', 'dialog', 'picture', 'source', 'video', 'time',
             'strong', 'em', 'small', 'mark', 'code', 'pre', 'blockquote', 'hr',
             'progress', 'meter', 'output', 'template', 'slot']

HTML_ATTRS = [
    lambda: f'class="{random.choice(COMPONENT_PREFIXES)}{random.choice(COMPONENT_SUFFIXES)}"',
    lambda: f'id="{random.choice(COMPONENT_PREFIXES)}-{random.randint(1,999)}"',
    lambda: f'data-{random.choice(["id","type","state","index","action","target","ref","key","value"])}="{random.choice(JS_VAR_PREFIXES)}{random.choice(JS_NOUNS)}"',
    lambda: f'aria-{random.choice(["label","hidden","expanded","selected","disabled","live","controls"])}="{random.choice(["true","false",random.choice(JS_NOUNS)])}"',
    lambda: f'role="{random.choice(["button","dialog","alert","tab","tabpanel","menu","menuitem","navigation","banner","complementary","contentinfo","main","search","form","group","list","listitem","region","status","timer","tooltip","tree","treeitem"])}"',
    lambda: f'tabindex="{random.choice(["0","-1","1"])}"',
    lambda: f'style="{random.choice(CSS_PROPS[:10])}:{css_value(random.choice(CSS_PROPS[:10]))}"',
    lambda: f'title="{random.choice(JS_VAR_PREFIXES)} {random.choice(JS_NOUNS).lower()}"',
]

HTML_TEXTS = [
    'Dashboard Overview', 'Settings & Preferences', 'Account Management',
    'User Profile', 'Notifications', 'Search Results', 'Recent Activity',
    'Quick Actions', 'Data Analytics', 'System Status', 'Performance Metrics',
    'Resource Monitor', 'Event Timeline', 'Audit Log', 'Access Control',
    'Integration Hub', 'API Documentation', 'Release Notes', 'Change History',
    'Team Management', 'Project Overview', 'Task Board', 'Sprint Planning',
    'Code Review', 'Deploy Pipeline', 'Error Tracking', 'Incident Response',
    'Service Health', 'Network Status', 'Storage Usage', 'Billing Summary',
    'Feature Flags', 'A/B Testing', 'Content Management', 'Media Library',
    'Email Templates', 'Notification Rules', 'Webhook Config', 'OAuth Settings',
    'Two-Factor Auth', 'Session Management', 'API Keys', 'Rate Limiting',
    'Caching Policy', 'CDN Configuration', 'SSL Certificates', 'DNS Records',
    'Load Balancer', 'Auto Scaling', 'Container Orchestration', 'Log Aggregation',
]


def gen_html_element(depth=0, max_depth=4):
    """Generate an HTML element with optional nesting."""
    tag = random.choice(HTML_TAGS)
    attrs = ' '.join(random.choice(HTML_ATTRS)() for _ in range(random.randint(1, 4)))
    void_tags = {'img', 'input', 'hr', 'br', 'source', 'meta', 'link'}
    if tag in void_tags:
        return f'<{tag} {attrs}/>'
    if depth >= max_depth or random.random() < 0.3:
        text = random.choice(HTML_TEXTS)
        return f'<{tag} {attrs}>{text}</{tag}>'
    children = ''.join(gen_html_element(depth+1, max_depth) for _ in range(random.randint(1, 4)))
    return f'<{tag} {attrs}>{children}</{tag}>'


def generate_html(target_kb, kind='header'):
    """Generate HTML content targeting approximately target_kb kilobytes."""
    parts = ['<!DOCTYPE html><html lang="en"><head><meta charset="utf-8">']
    parts.append('<meta name="viewport" content="width=device-width,initial-scale=1">')
    parts.append(f'<title>{random.choice(HTML_TEXTS)}</title>')
    for i in range(3):
        parts.append(f'<link rel="stylesheet" href="/static/{random.choice(["reset","layout","theme","components","utilities"])}.css">')
    parts.append('</head><body>')
    target_bytes = target_kb * 1024
    while len(''.join(parts)) < target_bytes - 50:
        parts.append(gen_html_element(0, 5))
    parts.append('</body></html>')
    return ''.join(parts)[:target_bytes]


# --- SVG generation ---

def gen_svg_path():
    """Generate a realistic SVG path data string."""
    cmds = []
    x, y = random.randint(2, 20), random.randint(2, 20)
    cmds.append(f'M{x} {y}')
    for _ in range(random.randint(5, 20)):
        cmd = random.choice(['L', 'l', 'C', 'c', 'Q', 'q', 'A', 'a', 'H', 'h', 'V', 'v', 'Z'])
        if cmd in ('Z', 'z'):
            cmds.append('Z')
            break
        elif cmd in ('H', 'h'):
            cmds.append(f'{cmd}{random.randint(-10, 10)}')
        elif cmd in ('V', 'v'):
            cmds.append(f'{cmd}{random.randint(-10, 10)}')
        elif cmd in ('L', 'l'):
            cmds.append(f'{cmd}{random.randint(0, 22)} {random.randint(0, 22)}')
        elif cmd in ('C', 'c'):
            pts = ' '.join(f'{random.randint(0, 24)} {random.randint(0, 24)}' for _ in range(3))
            cmds.append(f'{cmd}{pts}')
        elif cmd in ('Q', 'q'):
            pts = ' '.join(f'{random.randint(0, 24)} {random.randint(0, 24)}' for _ in range(2))
            cmds.append(f'{cmd}{pts}')
        elif cmd in ('A', 'a'):
            rx, ry = random.randint(2, 12), random.randint(2, 12)
            cmds.append(f'{cmd}{rx} {ry} 0 {random.choice([0,1])} {random.choice([0,1])} {random.randint(0, 22)} {random.randint(0, 22)}')
    if cmds[-1] != 'Z':
        cmds.append('Z')
    return ''.join(cmds)


def gen_svg_icon(name):
    """Generate an SVG icon symbol."""
    paths = ''.join(f'<path d="{gen_svg_path()}" fill="none" stroke="currentColor" stroke-width="{random.choice(["1.5","2","2.5"])}" stroke-linecap="round" stroke-linejoin="round"/>'
                    for _ in range(random.randint(1, 4)))
    extra = ''
    if random.random() < 0.3:
        cx, cy, r = random.randint(4, 20), random.randint(4, 20), random.randint(2, 8)
        extra = f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="currentColor" stroke-width="2"/>'
    if random.random() < 0.2:
        x, y, w, h = random.randint(2, 8), random.randint(2, 8), random.randint(6, 16), random.randint(6, 16)
        rx = random.choice([0, 1, 2, 3])
        extra += f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="none" stroke="currentColor" stroke-width="2"/>'
    if random.random() < 0.15:
        pts = ' '.join(f'{random.randint(2,22)},{random.randint(2,22)}' for _ in range(random.randint(3, 6)))
        extra += f'<polygon points="{pts}" fill="none" stroke="currentColor" stroke-width="2"/>'
    return f'<symbol id="{name}" viewBox="0 0 24 24">{paths}{extra}</symbol>'


def generate_icon_sprite(target_kb):
    """Generate an SVG icon sprite sheet."""
    parts = ['<svg xmlns="http://www.w3.org/2000/svg" style="display:none">']
    parts.append('<defs>')
    target_bytes = target_kb * 1024
    icons_used = set()
    while len(''.join(parts)) < target_bytes - 200:
        name = random.choice(SVG_ICON_NAMES)
        if name in icons_used:
            name = f'{name}-{random.choice(["alt","filled","outlined","rounded","sharp","thin","bold"])}'
        icons_used.add(name)
        parts.append(gen_svg_icon(name))
    parts.append('</defs></svg>')
    return ''.join(parts)[:target_bytes]


def generate_logo_svg(target_kb):
    """Generate a detailed SVG logo."""
    parts = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 60">']
    # Background
    parts.append(f'<rect width="200" height="60" rx="8" fill="{random.choice(COLORS)}"/>')
    target_bytes = target_kb * 1024
    while len(''.join(parts)) < target_bytes - 50:
        r = random.random()
        if r < 0.3:
            parts.append(f'<path d="{gen_svg_path()}" fill="{random.choice(COLORS)}" opacity="{random.choice([".3",".5",".7",".8","1"])}"/>')
        elif r < 0.5:
            cx, cy, rx, ry = random.randint(10, 190), random.randint(5, 55), random.randint(3, 20), random.randint(3, 15)
            parts.append(f'<ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}" fill="{random.choice(COLORS)}" opacity=".6"/>')
        elif r < 0.7:
            x, y = random.randint(10, 150), random.randint(10, 50)
            parts.append(f'<text x="{x}" y="{y}" font-family="sans-serif" font-size="{random.randint(10, 28)}" font-weight="{random.choice(["400","600","700","800"])}" fill="{random.choice(COLORS)}">{random.choice(["Arena","HTTP","Web","App","Hub","Dev","Pro","Lab"])}</text>')
        else:
            x, y, w, h = random.randint(5, 160), random.randint(5, 45), random.randint(10, 40), random.randint(5, 20)
            parts.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{random.randint(0,6)}" fill="{random.choice(COLORS)}" opacity=".5"/>')
    parts.append('</svg>')
    return ''.join(parts)[:target_bytes]


def generate_manifest(target_kb):
    """Generate a web app manifest."""
    icons = [{"src": f"/icons/icon-{s}x{s}.png", "sizes": f"{s}x{s}", "type": "image/png", "purpose": random.choice(["any","maskable","any maskable"])}
             for s in [48, 72, 96, 128, 144, 152, 192, 256, 384, 512]]
    shortcuts = [{"name": t, "short_name": t.split()[0], "url": f"/{t.lower().replace(' ','-')}", "description": f"Go to {t}"}
                 for t in random.sample(HTML_TEXTS, 6)]
    screenshots = [{"src": f"/screenshots/screen-{i}.png", "sizes": f"{random.choice(['1280x720','1920x1080','750x1334'])}",
                    "type": "image/png", "form_factor": random.choice(["wide","narrow"])} for i in range(4)]
    manifest = {
        "name": "HttpArena Web Application",
        "short_name": "HttpArena",
        "description": "A comprehensive HTTP framework benchmarking platform for comparing web server performance across multiple protocols and configurations",
        "start_url": "/",
        "scope": "/",
        "display": "standalone",
        "orientation": "any",
        "theme_color": "#1a1a2e",
        "background_color": "#ffffff",
        "categories": ["developer tools", "productivity", "utilities"],
        "lang": "en-US",
        "dir": "ltr",
        "icons": icons,
        "shortcuts": shortcuts,
        "screenshots": screenshots,
        "prefer_related_applications": False,
        "related_applications": [
            {"platform": "play", "url": "https://play.google.com/store/apps/details?id=com.httparena.app", "id": "com.httparena.app"},
            {"platform": "itunes", "url": "https://apps.apple.com/app/httparena/id123456789"}
        ],
        "protocol_handlers": [
            {"protocol": "web+httparena", "url": "/protocol?uri=%s"}
        ],
        "share_target": {
            "action": "/share",
            "method": "POST",
            "enctype": "multipart/form-data",
            "params": {"title": "title", "text": "text", "url": "url"}
        }
    }
    return json.dumps(manifest, separators=(',', ':'))


# --- Binary file generation ---

def generate_webp(target_kb):
    """Generate a valid WebP file of approximately target_kb."""
    # WebP file format: RIFF header + VP8 lossy bitstream
    target_bytes = target_kb * 1024
    # Use VP8L (lossless) format which is simpler to generate
    # RIFF header
    data = bytearray()

    # For a valid but simple WebP, we'll create a VP8 (lossy) container
    # with enough filler data to reach our target size
    width = random.randint(800, 1920)
    height = random.randint(400, 1080)

    # VP8 bitstream - minimal valid frame
    vp8_data = bytearray()
    # Frame tag (keyframe)
    frame_tag = (0 & 0x1) | ((0 & 0x7) << 1) | ((1 & 0x1) << 4)  # keyframe, version 0, show_frame
    size_part = len(b'\x9d\x01\x2a') + 4  # start code + dimensions placeholder
    vp8_data.append(frame_tag & 0xff)
    vp8_data.append((frame_tag >> 8) & 0xff)
    vp8_data.append((frame_tag >> 16) & 0xff)
    # VP8 start code
    vp8_data.extend(b'\x9d\x01\x2a')
    # Width and height (little-endian, 14 bits each + scale)
    vp8_data.extend(struct.pack('<H', width & 0x3fff))
    vp8_data.extend(struct.pack('<H', height & 0x3fff))
    # Fill with random data to simulate compressed image data
    remaining = target_bytes - 20 - len(vp8_data)  # 20 = RIFF(4) + size(4) + WEBP(4) + VP8_(4) + chunk_size(4)
    vp8_data.extend(random.randbytes(max(remaining, 100)))

    # Build RIFF container
    chunk_data = b'VP8 ' + struct.pack('<I', len(vp8_data)) + bytes(vp8_data)
    riff_data = b'WEBP' + chunk_data
    data = b'RIFF' + struct.pack('<I', len(riff_data)) + riff_data

    return bytes(data[:target_bytes])


def generate_woff2(target_kb):
    """Generate a realistic-sized binary blob mimicking woff2 structure."""
    target_bytes = target_kb * 1024
    # WOFF2 header
    data = bytearray()
    data.extend(b'wOF2')  # signature
    data.extend(struct.pack('>I', 0x00010000))  # flavor (TrueType)
    data.extend(struct.pack('>I', target_bytes))  # length
    data.extend(struct.pack('>H', 9))  # numTables
    data.extend(struct.pack('>H', 0))  # reserved
    data.extend(struct.pack('>I', target_bytes))  # totalSfntSize
    data.extend(struct.pack('>I', 0))  # totalCompressedSize
    data.extend(struct.pack('>H', 1))  # majorVersion
    data.extend(struct.pack('>H', 0))  # minorVersion
    data.extend(struct.pack('>I', 0))  # metaOffset
    data.extend(struct.pack('>I', 0))  # metaLength
    data.extend(struct.pack('>I', 0))  # metaOrigLength
    data.extend(struct.pack('>I', 0))  # privOffset
    data.extend(struct.pack('>I', 0))  # privLength
    # Fill remaining with random data
    remaining = target_bytes - len(data)
    data.extend(random.randbytes(max(remaining, 0)))
    return bytes(data[:target_bytes])


# --- Main generation ---

def compress_and_save(filepath, content):
    """Save file and its compressed variants."""
    if isinstance(content, str):
        data = content.encode('utf-8')
    else:
        data = content
    with open(filepath, 'wb') as f:
        f.write(data)
    # gzip level 9
    gz_path = filepath + '.gz'
    with open(gz_path, 'wb') as f:
        f.write(gzip.compress(data, compresslevel=9))
    # brotli level 11
    if HAS_BROTLI:
        br_path = filepath + '.br'
        with open(br_path, 'wb') as f:
            f.write(brotli.compress(data, quality=11))
    orig_size = len(data)
    gz_size = os.path.getsize(gz_path)
    br_size = os.path.getsize(br_path) if HAS_BROTLI else 0
    gz_ratio = (1 - gz_size / orig_size) * 100 if orig_size else 0
    br_ratio = (1 - br_size / orig_size) * 100 if orig_size and HAS_BROTLI else 0
    name = os.path.basename(filepath)
    print(f'  {name:30s} {orig_size:>8,}  gz:{gz_size:>8,} ({gz_ratio:.0f}%)  br:{br_size:>8,} ({br_ratio:.0f}%)')


def save_binary(filepath, content):
    """Save binary file without compression."""
    with open(filepath, 'wb') as f:
        f.write(content)
    name = os.path.basename(filepath)
    print(f'  {name:30s} {len(content):>8,}  (binary, no pre-compression)')


def main():
    os.makedirs(OUT, exist_ok=True)
    if not HAS_BROTLI:
        print('WARNING: brotli module not found, .br files will not be generated')
        print('  Install with: pip install brotli')
        return

    print('Generating static files...\n')
    print(f'  {"File":30s} {"Original":>8s}  {"Gzip-9":>14s}  {"Brotli-11":>14s}')
    print(f'  {"-"*30} {"-"*8}  {"-"*14}  {"-"*14}')

    # CSS files
    compress_and_save(os.path.join(OUT, 'reset.css'),
                      generate_css(8, include_vars=True))
    compress_and_save(os.path.join(OUT, 'layout.css'),
                      generate_css(20, include_media=True))
    compress_and_save(os.path.join(OUT, 'theme.css'),
                      generate_css(15, include_vars=True, include_keyframes=True))
    compress_and_save(os.path.join(OUT, 'components.css'),
                      generate_css(55, include_media=True, include_keyframes=True))
    compress_and_save(os.path.join(OUT, 'utilities.css'),
                      generate_css(45, include_media=True))

    # JS files
    compress_and_save(os.path.join(OUT, 'analytics.js'), generate_js(15))
    compress_and_save(os.path.join(OUT, 'helpers.js'), generate_js(25))
    compress_and_save(os.path.join(OUT, 'app.js'), generate_js(250))
    compress_and_save(os.path.join(OUT, 'vendor.js'), generate_js(400))
    compress_and_save(os.path.join(OUT, 'router.js'), generate_js(50))

    # HTML files
    compress_and_save(os.path.join(OUT, 'header.html'), generate_html(8, 'header'))
    compress_and_save(os.path.join(OUT, 'footer.html'), generate_html(5, 'footer'))

    # SVG files
    compress_and_save(os.path.join(OUT, 'icon-sprite.svg'), generate_icon_sprite(55))
    compress_and_save(os.path.join(OUT, 'logo.svg'), generate_logo_svg(12))

    # JSON manifest
    compress_and_save(os.path.join(OUT, 'manifest.json'), generate_manifest(2))

    # Binary files
    save_binary(os.path.join(OUT, 'hero.webp'), generate_webp(120))
    save_binary(os.path.join(OUT, 'thumb1.webp'), generate_webp(20))
    save_binary(os.path.join(OUT, 'thumb2.webp'), generate_webp(15))
    save_binary(os.path.join(OUT, 'regular.woff2'), generate_woff2(20))
    save_binary(os.path.join(OUT, 'bold.woff2'), generate_woff2(25))

    # Summary
    print()
    text_total = sum(os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT)
                     if not f.endswith(('.gz', '.br', '.webp', '.woff2')))
    binary_total = sum(os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT)
                       if f.endswith(('.webp', '.woff2')) and not f.endswith(('.gz', '.br')))
    gz_total = sum(os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT) if f.endswith('.gz'))
    br_total = sum(os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT) if f.endswith('.br'))
    print(f'  Text total:   {text_total:>10,} bytes ({text_total/1024:.0f} KB)')
    print(f'  Binary total: {binary_total:>10,} bytes ({binary_total/1024:.0f} KB)')
    print(f'  Grand total:  {(text_total+binary_total):>10,} bytes ({(text_total+binary_total)/1024:.0f} KB)')
    print(f'  Gzip total:   {gz_total:>10,} bytes ({gz_total/1024:.0f} KB) — {(1-gz_total/text_total)*100:.0f}% reduction')
    print(f'  Brotli total: {br_total:>10,} bytes ({br_total/1024:.0f} KB) — {(1-br_total/text_total)*100:.0f}% reduction')


if __name__ == '__main__':
    main()
