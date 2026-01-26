#!/usr/bin/env python3

import os
import re
import argparse

def count_effective_lines(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    # 步骤1：找到测试模块的位置，只统计这个位置之前的行
    # 寻找所有#[cfg(test)]行，然后检查下一行是否是mod tests
    test_module_start = len(lines)
    
    # 遍历所有行
    for i in range(len(lines)):
        line = lines[i]
        
        # 检查是否是#[cfg(test)]行
        if '#[cfg(test)]' in line:
            # 确保不是在字符串内
            pre_content = ''.join(lines[:i])
            # 只检查双引号，因为Rust中字符使用单引号，不一定成对出现
            double_quotes_ok = pre_content.count('"') % 2 == 0
            
            if double_quotes_ok:
                # 检查下一行是否存在
                if i + 1 < len(lines):
                    next_line = lines[i+1]
                    # 检查下一行是否是mod tests
                    if next_line.strip().startswith('mod tests'):
                        # 更新测试模块位置，选择最后一个出现的
                        test_module_start = i
    
    # 步骤2：在测试模块之前的内容中，统计有效代码行数（排除pub mod和mod声明）
    effective_lines_count = 0
    
    for i in range(test_module_start):
        line = lines[i]
        # 跳过空行
        if not line.strip():
            continue
        
        # 检查是否是mod或pub mod声明
        if re.match(r'\s*(pub\s+)?mod\s+', line):
            # 确保不是在字符串内
            pre_content = ''.join(lines[:i])
            if pre_content.count('"') % 2 == 0 and pre_content.count("'") % 2 == 0:
                # 不是在字符串内，这是一个有效的mod声明，跳过这一行
                continue
        
        # 这是一行有效代码
        effective_lines_count += 1
    
    return effective_lines_count

def main():
    # 创建参数解析器
    parser = argparse.ArgumentParser(description='统计Rust项目的有效代码行数（排除测试代码）')
    parser.add_argument('directory', nargs='?', default='.', help='要扫描的Rust项目目录（默认：当前目录）')
    
    # 解析命令行参数
    args = parser.parse_args()
    
    rust_files = []
    
    # 遍历指定目录下的所有 Rust 文件，排除 target 目录
    scan_dir = args.directory
    
    # 确保目录路径是绝对路径
    if not os.path.isabs(scan_dir):
        scan_dir = os.path.abspath(scan_dir)
    
    for root, dirs, files in os.walk(scan_dir):
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