# Wrap UI string literals in tr(...). Prints each change for review.
import re, sys
STR = re.compile(r'"((?:[^"\\]|\\.)*)"')
RAW = re.compile(r'r#"(?:.|\n)*?"#', re.S)
SKIP_BEFORE = re.compile(r'(\b(?:format|println|eprintln|bail|panic|write|writeln|matches|assert|assert_eq|anyhow)!\(\s*$|'
    r'(?:\.tag|add_css_class|remove_css_class|has_css_class|from_icon_name|icon_name|set_icon_name|css_classes|widget_name|set_widget_name|env::var|var_os|expect|tr|trf|n_|spawn_detached|Command::new|\.arg|args|\.get|starts_with|ends_with|strip_prefix|strip_suffix|contains|split|join|replace|property|connect_notify_local|set_property|lookup|Variant::from|set_string|set_boolean|value|child_by_name|visible_child_name|set_visible_child_name|add_named|action_name|set_action_name|SimpleAction::new|activate_action|set_accels_for_action|application_id)\(\s*$|'
    r'==\s*$|!=\s*$|=>\s*$|\|\s*$|&\[\s*$)')
def ui_like(s):
    if not re.search(r'[A-Za-z]', s): return False
    if s.endswith('-symbolic') or s.startswith(('/', 'com.', 'org.', 'file:', 'http', '.', '-', '$', '%')): return False
    if re.fullmatch(r'[a-z0-9_.:+/-]*', s): return False          # ids, css classes, keys
    if re.fullmatch(r'[A-Z0-9_]+', s): return False                # env vars, consts
    if '/' in s and ' ' not in s: return False
    if re.fullmatch(r'\{[^}]*\}', s): return False
    if ' ' in s or '\\n' in s: return True
    return bool(re.fullmatch(r'_?[A-Z][a-z]+[.!?…]?', s)) or s in ('MAN',)
for path in sys.argv[1:]:
    src = open(path).read()
    raws = [m.span() for m in RAW.finditer(src)]
    out, last, n = [], 0, 0
    for m in STR.finditer(src):
        a, b = m.span()
        if any(x <= a < y for x, y in raws): continue
        if src[a-1:a] in ('r', 'b', '#', "'"): continue
        lit = m.group(1)
        line_start = src.rfind('\n', 0, a) + 1
        line = src[line_start:src.find('\n', a)]
        if line.lstrip().startswith(('//', '#[', '///')): continue
        before = src[max(0, a-60):a]
        if SKIP_BEFORE.search(before): continue
        if 'const ' in line or 'static ' in line: continue
        if not ui_like(lit): continue
        out.append(src[last:a]); out.append(f'tr("{lit}")'); last = b; n += 1
        print(f'{path}:{src.count(chr(10),0,a)+1}: {lit[:90]}')
    out.append(src[last:])
    open(path, 'w').write(''.join(out))
    print(f'## {path}: {n} wrapped', file=sys.stderr)
