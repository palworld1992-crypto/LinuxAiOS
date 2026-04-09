
#!/usr/bin/env python3
"""
check_forbidden.py - Kiểm tra code Rust/Zig/Ada/C++ theo quy tắc AIOS
Hỗ trợ bỏ qua thư mục/file thông qua .checkignore
Chạy: python3 check_forbidden.py
"""

import re
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Set

RED = '\033[0;31m'
GREEN = '\033[0;32m'
YELLOW = '\033[0;33m'
NC = '\033[0m'

# Danh sách mặc định các thư mục và file bị bỏ qua
DEFAULT_IGNORE_DIRS = {
    'target', '.git', '.cargo', 'zig-out', 'obj', 'alire',
    'node_modules', 'dist', 'build', '__pycache__', '.idea', '.rustup', '.vscode'
}
DEFAULT_IGNORE_FILES = {
    'cbindgen.toml', 'Cargo.lock', 'Cargo.toml', 'build.rs',
    'README.md', '.gitignore', '.gitmodules', 'LICENSE'
}

# Các pattern cho phép (TODO, unimplemented, panic, v.v.)
ALLOWED_PATTERNS = [
    r"//\s*(TODO|FIXME|HACK|XXX).*Phase",
    r"unimplemented!\s*\(",
    r"todo!\s*\(",
    r"panic!\s*\(",
    r"@panic\(",
    r"raise\s+Program_Error",
    r"pragma\s+Assert\s*\(\s*False",
    r"std::abort\s*\(",
    r"throw\s+std::logic_error",
    r"//\s*(fallback|simulated|mock|dummy|placeholder|not supported)",
]

# Các pattern cấm (giá trị giả)
FORBIDDEN_PATTERNS = [
    (re.compile(r'\bOk\(\(\)\)\b'), "Ok(())"),
    (re.compile(r'\bOk\(None\)\b'), "Ok(None)"),
    (re.compile(r'\bOk\(0\)\b'), "Ok(0)"),
    (re.compile(r'\bOk\(vec!\?\[\]\)\b'), "Ok(vec![])"),
    (re.compile(r'\bOk\(Vec::new\(\)\)\b'), "Ok(Vec::new())"),
    (re.compile(r'\bvec!\?\[\]\b'), "vec![]"),
    (re.compile(r'\bNone\b'), "None"),
    (re.compile(r'\bSome\(0\)\b'), "Some(0)"),
    (re.compile(r'\bSome\(vec!\?\[\]\)\b'), "Some(vec![])"),
    (re.compile(r'\bfalse\b'), "false"),
    (re.compile(r'\btrue\b'), "true"),
    (re.compile(r'\b0\b'), "0"),
    (re.compile(r'""'), '""'),
    (re.compile(r'\bString::new\(\)\b'), "String::new()"),
    (re.compile(r'\bDefault::default\(\)\b'), "Default::default()"),
    (re.compile(r'\breturn\s+0\s*;'), "return 0;"),
    (re.compile(r'\breturn\s+false\s*;'), "return false;"),
    (re.compile(r'\breturn\s+null\s*;'), "return null;"),
    (re.compile(r'\breturn\s+\.\{\}\s*;'), "return .{};"),
    (re.compile(r'\breturn\s*;'), "return;"),
]

def load_ignore_patterns(root: Path) -> Tuple[Set[str], Set[str]]:
    """Đọc file .checkignore (nếu có) để bổ sung danh sách bỏ qua"""
    ignore_dirs = DEFAULT_IGNORE_DIRS.copy()
    ignore_files = DEFAULT_IGNORE_FILES.copy()
    ignore_file_path = root / '.checkignore'
    if ignore_file_path.exists():
        try:
            for line in ignore_file_path.read_text(encoding='utf-8').splitlines():
                line = line.strip()
                if not line or line.startswith('#'):
                    continue
                if line.endswith('/'):
                    ignore_dirs.add(line.rstrip('/'))
                else:
                    ignore_files.add(line)
        except Exception as e:
            print(f"{YELLOW}⚠️ Không thể đọc .checkignore: {e}{NC}", file=sys.stderr)
    return ignore_dirs, ignore_files

def should_ignore(path: Path, ignore_dirs: Set[str], ignore_files: Set[str]) -> bool:
    """Kiểm tra xem file/thư mục có bị bỏ qua không"""
    if path.name in ignore_files:
        return True
    for part in path.parts:
        if part in ignore_dirs:
            return True
    return False

def is_test_file(path: Path) -> bool:
    """Xác định file test (thư mục tests/, fuzz/, hoặc có #[cfg(test)]/mod tests)"""
    if 'tests' in path.parts or 'fuzz' in path.parts:
        return True
    try:
        content = path.read_text(encoding='utf-8', errors='ignore')
        if '#[cfg(test)]' in content or 'mod tests {' in content:
            return True
    except:
        pass
    return False

def has_allowed_comment(line: str, prev_lines: List[str]) -> bool:
    """Kiểm tra dòng hoặc 2 dòng trước có chứa pattern cho phép không"""
    context = prev_lines[-2:] + [line]
    for l in context:
        for pat in ALLOWED_PATTERNS:
            if re.search(pat, l):
                return True
    return False

def check_file(path: Path, errors: Dict[str, List[Tuple[str, int, str]]], ignore_dirs: Set[str], ignore_files: Set[str]):
    if should_ignore(path, ignore_dirs, ignore_files):
        return
    is_test = is_test_file(path)
    category = 'test' if is_test else 'production'
    try:
        lines = path.read_text(encoding='utf-8', errors='ignore').splitlines()
        content = '\n'.join(lines)
    except:
        return

    # Duyệt từng dòng
    for i, line in enumerate(lines, 1):
        # Bỏ qua nếu dòng có comment cho phép
        if has_allowed_comment(line, lines[max(0,i-3):i]):
            continue

        # 1. unwrap() và expect()
        if '.unwrap()' in line:
            errors[category].append((str(path), i, ".unwrap()"))
        if '.expect(' in line:
            errors[category].append((str(path), i, ".expect("))

        # 2. unwrap_or, unwrap_or_else, unwrap_or_default
        if re.search(r'\.unwrap_or\(', line):
            errors[category].append((str(path), i, ".unwrap_or("))
        if re.search(r'\.unwrap_or_else\(', line):
            errors[category].append((str(path), i, ".unwrap_or_else("))
        if '.unwrap_or_default()' in line:
            errors[category].append((str(path), i, ".unwrap_or_default()"))

        # 3. #[allow(...)] và #![allow(...)]
        if re.search(r'#\[allow\(', line):
            errors[category].append((str(path), i, "#[allow(...)]"))
        if re.search(r'#!\[allow\(', line):
            errors[category].append((str(path), i, "#![allow(...)]"))

        # 4. println! và eprintln!
        if 'println!' in line:
            errors[category].append((str(path), i, "println! (dùng tracing)"))
        if 'eprintln!' in line:
            errors[category].append((str(path), i, "eprintln! (dùng tracing)"))

        # 5. todo!
        if 'todo!' in line:
            errors[category].append((str(path), i, "todo! (dùng unimplemented! với TODO)"))

        # 6. Mutex<HashMap> / RwLock<HashMap> trong production
        if not is_test:
            if re.search(r'(Mutex|RwLock)<(HashMap|HashSet)', line):
                errors[category].append((str(path), i, "Dùng Mutex/RwLock thay vì DashMap"))

        # 7. SQLite trong luồng chính
        if not is_test:
            if 'rusqlite::Connection' in line and 'thread::spawn' not in content and 'tokio::spawn' not in content:
                if re.search(r'\.query\(|\.execute\(', line):
                    errors[category].append((str(path), i, "SQLite query trong luồng chính (cần background thread)"))

        # 8. unsafe thiếu comment SAFETY
        if 'unsafe {' in line:
            has_safety = False
            for j in range(max(0, i-3), i):
                if 'SAFETY:' in lines[j]:
                    has_safety = True
                    break
            if not has_safety:
                errors[category].append((str(path), i, "unsafe thiếu comment // SAFETY: ..."))

        # 9. FFI extern "C" không có catch_unwind
        if 'extern "C"' in line and 'catch_unwind' not in content:
            errors[category].append((str(path), i, "FFI extern \"C\" cần bọc catch_unwind"))

        # 10. rand::thread_rng() cho crypto
        if 'rand::thread_rng()' in line:
            errors[category].append((str(path), i, "Dùng rand::thread_rng() cho crypto (cần OsRng)"))

        # 11. Gọi shell trực tiếp
        if 'Command::new("sh")' in line or 'Command::new("bash")' in line or 'system(' in line:
            errors[category].append((str(path), i, "Gọi shell trực tiếp (cấm)"))

        # 12. #[cfg(not(feature = "mock"))] trong production
        if '#[cfg(not(feature = "mock"))]' in line and not is_test:
            errors[category].append((str(path), i, "Dùng #[cfg(not(feature = \"mock\"))] trong production"))

    # 13. Kiểm tra giá trị giả trong toàn bộ file (không theo dòng)
    # Hàm trả về Ok(()) giả
    for match in re.finditer(r'(pub\s+)?(async\s+)?fn\s+\w+.*->\s*Result<[^>]+>', content):
        start = match.start()
        brace_count = 0
        end = start
        while end < len(content):
            if content[end] == '{':
                brace_count += 1
            elif content[end] == '}':
                brace_count -= 1
                if brace_count == 0:
                    break
            end += 1
        body = content[start:end+1]
        if re.search(r'Ok\(\(\)\)\s*$', body, re.MULTILINE):
            errors[category].append((str(path), 0, "Hàm trả về Ok(()) giả"))
        if 'Ok(Default::default())' in body:
            errors[category].append((str(path), 0, "Hàm trả về Ok(Default::default()) giả"))

    # vec![] giả
    if re.search(r'fn.*->\s*Vec<[^>]+>', content):
        for match in re.finditer(r'vec!\?\[\]', content):
            ctx = content[max(0, match.start()-50):match.end()+50]
            if not re.search(r'unimplemented|todo', ctx):
                errors[category].append((str(path), 0, "vec![] giả"))

    # None giả
    if re.search(r'fn.*->\s*Option<[^>]+>', content):
        for match in re.finditer(r'None', content):
            ctx = content[max(0, match.start()-50):match.end()+50]
            if not re.search(r'unimplemented|todo', ctx):
                errors[category].append((str(path), 0, "None giả"))

    # return 0 giả
    if re.search(r'fn.*->\s*(u32|u64|usize|i32|i64)\b', content):
        for match in re.finditer(r'return\s+0;', content):
            ctx = content[max(0, match.start()-50):match.end()+50]
            if not re.search(r'unimplemented|todo', ctx):
                errors[category].append((str(path), 0, "return 0 giả"))

def main():
    root = Path.cwd()
    ignore_dirs, ignore_files = load_ignore_patterns(root)
    errors = {'production': [], 'test': []}
    # Quét .rs, .zig, .adb, .ads, .cpp, .h, .hpp, .c, .cc
    extensions = {'.rs', '.zig', '.adb', '.ads', '.cpp', '.h', '.hpp', '.c', '.cc'}
    for path in root.rglob('*'):
        if path.suffix not in extensions:
            continue
        if should_ignore(path, ignore_dirs, ignore_files):
            continue
        check_file(path, errors, ignore_dirs, ignore_files)

    total = 0
    for category in ['production', 'test']:
        if errors[category]:
            print(f"\n{'='*60}")
            print(f"{YELLOW}🔴 LỖI TRONG CODE {category.upper()}{NC}")
            print(f"{'='*60}")
            for file, line, msg in errors[category]:
                if line:
                    print(f"  {file}:{line} - {msg}")
                else:
                    print(f"  {file} - {msg}")
            total += len(errors[category])

    if total == 0:
        print(f"{GREEN}✅ Không phát hiện vi phạm nào{NC}")
        sys.exit(0)
    else:
        print(f"\n{RED}❌ Tổng cộng {total} vi phạm (production: {len(errors['production'])}, test: {len(errors['test'])}). Cần sửa.{NC}")
        sys.exit(1)

if __name__ == '__main__':
    main()
