#!/usr/bin/env python3
import os
import re
import argparse

def run_rca_command(directory):
    """执行rust-code-analysis-cli命令获取代码分析结果"""
    os.system(f"rust-code-analysis-cli -p {directory} -F -m > result.txt")
    print("代码分析命令执行完成，结果已保存到result.txt")

def parse_rca_results(no_test=False, no_anonymous=False):
    """解析rust-code-analysis-cli命令的结果，提取每个文件中每个方法的长度和圈复杂度"""
    results = []
    
    # 读取result.txt文件
    with open("result.txt", "r", encoding="utf-8") as f:
        content = f.read()
    
    # 去除ANSI转义序列
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[.*?[a-zA-Z])')
    content = ansi_escape.sub('', content)
    
    # 分割内容为文件块
    file_blocks = re.findall(r"`- unit: (.+?) \(@\d+\)(.*?)(?=`- unit:|\Z)", content, re.DOTALL)
    
    # 处理每个文件块
    for file_name, file_content in file_blocks:
        # 记录已处理的函数位置（用于避免重复处理）
        processed_functions = set()
        
        # 1. 先处理impl块内的函数
        impl_blocks = re.findall(r"impl: (.+?) \(@\d+\)(.*?)(?=\s*`- unit:|\s*\|- impl:|\Z)", file_content, re.DOTALL)
        
        for struct_name, impl_content in impl_blocks:
            # 提取impl块内的函数
            function_blocks = re.findall(r"function: (.+?) \(@(\d+)\)(.*?)(?=function:|\Z)", impl_content, re.DOTALL)
            
            for func_name, start_line, block_content in function_blocks:
                # 提取圈复杂度
                cyclomatic_match = re.search(r"cyclomatic.*?sum: (\d+)", block_content, re.DOTALL)
                cyclomatic = int(cyclomatic_match.group(1)) if cyclomatic_match else 0
                
                # 提取代码行数
                loc_match = re.search(r"loc.*?sloc: (\d+)", block_content, re.DOTALL)
                lines_count = int(loc_match.group(1)) if loc_match else 0
                
                # 提取逻辑代码行数
                lloc_match = re.search(r"loc.*?lloc: (\d+)", block_content, re.DOTALL)
                logical_lines_count = int(lloc_match.group(1)) if lloc_match else 0
                
                # 计算结束行
                end_line = int(start_line) + lines_count - 1 if start_line and lines_count else 0
                
                # 过滤掉函数名为空的行
                if func_name.strip():
                    # 如果需要过滤测试函数，检查函数名是否以test_开头
                    if no_test and func_name.strip().startswith("test_"):
                        continue
                    # 如果需要过滤匿名函数，检查函数名是否为<anonymous>
                    if no_anonymous and func_name.strip() == "<anonymous>":
                        continue
                    
                    # 转义Markdown中的尖括号
                    escaped_func_name = func_name.replace("<", "&lt;").replace(">", "&gt;")
                    # 添加到结果
                    results.append({
                        "file": file_name,
                        "struct": struct_name.strip(),
                        "function": escaped_func_name,
                        "start_line": start_line,
                        "end_line": end_line,
                        "lines": lines_count,
                        "logical_lines": logical_lines_count,
                        "cyclomatic_complexity": cyclomatic
                    })
                    # 记录已处理的函数
                    processed_functions.add((func_name.strip(), start_line))
        
        # 2. 处理文件级别的普通函数
        # 提取所有函数，然后过滤掉已处理的impl块内函数
        all_functions = re.findall(r"function: (.+?) \(@(\d+)\)(.*?)(?=function:|\Z)", file_content, re.DOTALL)
        
        for func_name, start_line, block_content in all_functions:
            # 检查是否已处理过
            if (func_name.strip(), start_line) in processed_functions:
                continue
            
            # 提取圈复杂度
            cyclomatic_match = re.search(r"cyclomatic.*?sum: (\d+)", block_content, re.DOTALL)
            cyclomatic = int(cyclomatic_match.group(1)) if cyclomatic_match else 0
            
            # 提取代码行数
            loc_match = re.search(r"loc.*?sloc: (\d+)", block_content, re.DOTALL)
            lines_count = int(loc_match.group(1)) if loc_match else 0
            
            # 提取逻辑代码行数
            lloc_match = re.search(r"loc.*?lloc: (\d+)", block_content, re.DOTALL)
            logical_lines_count = int(lloc_match.group(1)) if lloc_match else 0
            
            # 计算结束行
            end_line = int(start_line) + lines_count - 1 if start_line and lines_count else 0
            
            # 过滤掉函数名为空的行
            if func_name.strip():
                # 如果需要过滤测试函数，检查函数名是否以test_开头
                if no_test and func_name.strip().startswith("test_"):
                    continue
                # 如果需要过滤匿名函数，检查函数名是否为<anonymous>
                if no_anonymous and func_name.strip() == "<anonymous>":
                    continue
                
                # 转义Markdown中的尖括号
                escaped_func_name = func_name.replace("<", "&lt;").replace(">", "&gt;")
                # 添加到结果（普通函数，struct为"-"）
                results.append({
                    "file": file_name,
                    "struct": "-",
                    "function": escaped_func_name,
                    "start_line": start_line,
                    "end_line": end_line,
                    "lines": lines_count,
                    "logical_lines": logical_lines_count,
                    "cyclomatic_complexity": cyclomatic
                })
    
    return results

def generate_markdown(results):
    """生成Markdown表格"""
    md_content = "# 代码分析结果\n\n"
    md_content += "| 文件 | 所属结构体 | 函数 | 起始行 | 结束行 | 代码行数 | 逻辑代码行数 | 圈复杂度 |\n"
    md_content += "|------|------------|------|--------|--------|----------|--------------|----------|\n"
    
    for result in results:
        md_content += f"| {result['file']} | {result['struct']} | {result['function']} | {result['start_line']} | {result['end_line']} | {result['lines']} | {result['logical_lines']} | {result['cyclomatic_complexity']} |\n"
    
    return md_content

def main():
    """主函数"""
    # 解析命令行参数
    parser = argparse.ArgumentParser(description="分析代码并生成包含方法长度和圈复杂度的Markdown表格")
    parser.add_argument("directory", nargs="?", default="src", help="要分析的目录路径，默认为src")
    parser.add_argument("--no-test", action="store_true", help="过滤掉测试函数")
    parser.add_argument("--no-anonymous", action="store_true", help="过滤掉匿名函数")
    args = parser.parse_args()
    
    # 执行rca命令
    run_rca_command(args.directory)
    
    # 解析结果
    results = parse_rca_results(args.no_test, args.no_anonymous)
    
    if not results:
        print("未解析到任何函数信息，请检查result.txt文件的格式")
        return
    
    # 生成Markdown表格
    md_content = generate_markdown(results)
    
    # 写入文件
    with open("rca_analysis.md", "w", encoding="utf-8") as f:
        f.write(md_content)
    
    print(f"分析完成，共处理 {len(results)} 个函数")
    print("结果已保存到rca_analysis.md")

if __name__ == "__main__":
    main()