import os
import yaml

def analyze_yml_files(directory):
    results = []
    
    # 遍历目录及其子目录
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith('.rs.yml'):
                file_path = os.path.join(root, file)
                with open(file_path, 'r', encoding='utf-8') as f:
                    try:
                        data = yaml.safe_load(f)
                        extract_functions(data, results, file_path)
                    except yaml.YAMLError as e:
                        print(f"Error parsing {file_path}: {e}")
    
    return results

def extract_functions(data, results, file_path):
    """递归提取所有函数信息"""
    if isinstance(data, dict):
        # 检查当前节点是否是函数
        if data.get('kind') == 'function':
            function_name = data.get('name', 'anonymous')
            start_line = data.get('start_line')
            end_line = data.get('end_line')
            
            # 提取度量信息
            metrics = data.get('metrics', {})
            cyclomatic = metrics.get('cyclomatic', {}).get('sum', 0)
            loc = metrics.get('loc', {})
            sloc = loc.get('sloc', 0)
            
            # 添加到结果
            results.append({
                'file': os.path.relpath(file_path, '/Users/treagzhao/Documents/Workspace/austrian_market_sim'),
                'function': function_name,
                'start_line': start_line,
                'end_line': end_line,
                'lines': sloc,
                'cyclomatic_complexity': cyclomatic
            })
        
        # 递归处理子节点
        for key, value in data.items():
            if key == 'spaces' and isinstance(value, list):
                for space in value:
                    extract_functions(space, results, file_path)

def generate_markdown(results):
    """生成 Markdown 表格"""
    md_content = "# 代码分析结果\n\n"
    md_content += "| 文件 | 函数 | 起始行 | 结束行 | 代码行数 | 圈复杂度 |\n"
    md_content += "|------|------|--------|--------|----------|----------|\n"
    
    for result in results:
        md_content += f"| {result['file']} | {result['function']} | {result['start_line']} | {result['end_line']} | {result['lines']} | {result['cyclomatic_complexity']} |\n"
    
    return md_content

def main():
    # 分析 src/model 目录
    model_directory = '/Users/treagzhao/Documents/Workspace/austrian_market_sim/src/model'
    results = analyze_yml_files(model_directory)
    
    # 生成 Markdown
    md_content = generate_markdown(results)
    
    # 写入文件
    with open('code_analysis.md', 'w', encoding='utf-8') as f:
        f.write(md_content)
    
    print(f"分析完成，共处理 {len(results)} 个函数")
    print("结果已保存到 code_analysis.md")

if __name__ == "__main__":
    main()
