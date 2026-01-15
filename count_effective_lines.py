import os
import re

def count_effective_lines(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 匹配 `#[cfg(test)]` 后接任意空白字符（包括换行）再后接 `mod ` 的模式
    pattern = r'#\[cfg\(test\)\]\s*\n\s*mod\s+'
    match = re.search(pattern, content)
    
    if match:
        # 计算匹配位置之前的行数
        effective_content = content[:match.start()]
        effective_lines = effective_content.count('\n') + 1
        return effective_lines
    
    # 如果没有找到匹配的测试代码标记，返回所有行数
    return content.count('\n') + 1

def main():
    rust_files = []
    
    # 遍历所有 Rust 文件，排除 target 目录
    for root, dirs, files in os.walk('.'):
        # 跳过 target 目录
        if 'target' in dirs:
            dirs.remove('target')
        
        for file in files:
            if file.endswith('.rs'):
                rust_files.append(os.path.join(root, file))
    
    total_lines = 0
    results = []
    
    # 统计每个文件的有效代码行数
    for file in rust_files:
        effective_lines = count_effective_lines(file)
        total_lines += effective_lines
        results.append((file, effective_lines))
    
    # 将结果写入文件
    with open('effective_code_lines.txt', 'w', encoding='utf-8') as f:
        f.write('文件路径,有效代码行数\n')
        for file, lines in sorted(results):
            f.write(f'{file},{lines}\n')
        f.write(f'\n总有效代码行数,{total_lines}\n')
    
    print('统计完成，结果已写入 effective_code_lines.txt')
    print(f'总有效代码行数: {total_lines}')

if __name__ == '__main__':
    main()