#!/usr/bin/env python3
"""
奥地利市场模拟器依赖分析工具

该工具执行 cargo modules dependencies 命令，分析项目的依赖关系，并生成详细的依赖报告。

用法:
    cargo-dep-analyzer [OPTIONS]

选项:
    -o, --output FILE  指定输出报告文件路径 (默认: architecture_dependency_report.md)
    -h, --help         显示此帮助信息并退出
"""

import re
import os
import sys
import argparse
from collections import defaultdict
from typing import Dict, Set, List, Tuple


def parse_structure_output(output: str) -> Tuple[Dict, Dict]:
    """解析 cargo modules dependencies 命令输出，提取节点和边信息。"""
    nodes = {}
    edges = []
    current_section = None

    lines = output.split('\n')

    for line in lines:
        line = line.strip()
        if not line or line.startswith('//') or line.startswith('/*'):
            continue

        if line.startswith('digraph') or line.startswith('graph'):
            continue

        if line.startswith('node ['):
            current_section = 'node'
            continue
        elif line.startswith('edge ['):
            current_section = 'edge'
            continue
        elif line.startswith('}'):
            break

        node_match = re.match(r'"([^"]+)"\s*\[(.*?)\]', line)
        if node_match:
            full_path = node_match.group(1)
            attrs = node_match.group(2)
            node_type = extract_node_type(attrs)
            nodes[full_path] = {
                'type': node_type,
                'full_path': full_path
            }
            continue

        edge_match = re.match(r'"([^"]+)"\s*->\s*"([^"]+)"\s*\[(.*?)\]', line)
        if edge_match:
            source = edge_match.group(1)
            target = edge_match.group(2)
            attrs = edge_match.group(3)
            edge_type = extract_edge_type(attrs)
            edges.append({
                'source': source,
                'target': target,
                'relation': edge_type
            })
            continue

    return nodes, edges


def extract_node_type(attrs: str) -> str:
    """从节点属性中提取类型。"""
    type_match = re.search(r'label="([^|]+)', attrs)
    if type_match:
        return type_match.group(1).strip()
    return 'unknown'


def extract_edge_type(attrs: str) -> str:
    """从边属性中提取关系类型。"""
    label_match = re.search(r'label="([^"]+)"', attrs)
    if label_match:
        return label_match.group(1)
    return 'uses'


def is_internal_path(path: str) -> bool:
    """判断路径是否属于当前仓库内部。"""
    return path.startswith('austrian_market_sim::')


def get_top_level_module(full_path: str) -> str:
    """获取顶级模块名。"""
    parts = full_path.split('::')
    if parts[0] == 'austrian_market_sim' and len(parts) > 1:
        return f'austrian_market_sim::{parts[1]}'
    return 'austrian_market_sim'


def get_file_path(full_path: str) -> str:
    """从完整路径中提取文件路径。"""
    parts = full_path.split('::')
    if parts[0] == 'austrian_market_sim':
        # 构建完整的模块路径，包括子模块
        module_parts = []
        
        # 处理所有部分，直到遇到方法名或结构体/枚举名
        for i, part in enumerate(parts[1:]):
            # 检查是否是方法名（小写开头且不是已知模块名）
            if part[0].islower() and part not in ['entity', 'normal_distribute', 'logging', 'model', 'agent', 'preference', 'factory', 'accountant', 'financial_bill', 'market', 'math', 'product', 'util', 'agent_cash_log', 'agent_demand_removal_log', 'agent_range_adjustment_log', 'factory_end_of_round_log', 'factory_range_optimization_log', 'trade_log']:
                break
            
            # 检查是否是结构体/枚举名（大写开头）
            if part[0].isupper():
                # 如果前面有模块名，使用前面的模块名
                if module_parts:
                    break
                else:
                    # 没有前面的模块名，可能是直接在 lib.rs 中定义的
                    return 'src/lib.rs'
            
            # 这是模块名
            module_parts.append(part)
        
        # 基于实际的 Rust 模块结构映射
        if not module_parts:
            return 'src/lib.rs'
        
        # 特殊映射规则
        module_path = '::'.join(module_parts)
        
        # 实体模块
        if module_path == 'entity::normal_distribute':
            return 'src/entity/normal_distribute.rs'
        elif module_path == 'entity':
            return 'src/entity.rs'
        
        # 日志模块 - 特别处理所有子模块
        elif module_path == 'logging':
            return 'src/logging.rs'
        elif module_path == 'logging::agent_cash_log':
            return 'src/logging/agent_cash_log.rs'
        elif module_path == 'logging::agent_demand_removal_log':
            return 'src/logging/agent_demand_removal_log.rs'
        elif module_path == 'logging::agent_range_adjustment_log':
            return 'src/logging/agent_range_adjustment_log.rs'
        elif module_path == 'logging::factory_end_of_round_log':
            return 'src/logging/factory_end_of_round_log.rs'
        elif module_path == 'logging::factory_range_optimization_log':
            return 'src/logging/factory_range_optimization_log.rs'
        elif module_path == 'logging::trade_log':
            return 'src/logging/trade_log.rs'
        elif module_path.startswith('logging::'):
            submodule = module_path.split('::')[1]
            return f'src/logging/{submodule}.rs'
        
        # 模型模块
        elif module_path == 'model':
            return 'src/model.rs'
        elif module_path == 'model::agent':
            return 'src/model/agent.rs'
        elif module_path == 'model::agent::preference':
            return 'src/model/agent/preference.rs'
        elif module_path == 'model::factory':
            return 'src/model/factory.rs'
        elif module_path == 'model::factory::accountant':
            return 'src/model/factory/accountant.rs'
        elif module_path == 'model::factory::financial_bill':
            return 'src/model/factory/financial_bill.rs'
        elif module_path == 'model::market':
            return 'src/model/market.rs'
        elif module_path == 'model::math':
            return 'src/model/math.rs'
        elif module_path == 'model::product':
            return 'src/model/product.rs'
        
        # 工具模块
        elif module_path == 'util':
            return 'src/util.rs'
        
        # 其他情况
        return 'src/lib.rs'
    return 'src/lib.rs'


def extract_class_name(full_path: str) -> str:
    """从完整路径中提取类/结构体/枚举名。"""
    parts = full_path.split('::')
    return parts[-1] if parts else full_path


def filter_internal_dependencies(nodes: Dict, edges: List[Dict]) -> Tuple[Dict, List[Dict]]:
    """筛选出仓库内部的依赖关系。"""
    internal_nodes = {k: v for k, v in nodes.items() if is_internal_path(k)}
    internal_edges = [e for e in edges if is_internal_path(e['source']) and is_internal_path(e['target'])]
    return internal_nodes, internal_edges


def build_class_dependencies(edges: List[Dict], nodes: Dict) -> Dict[str, Dict[str, Set[str]]]:
    """构建每个类的依赖关系。"""
    class_deps = defaultdict(lambda: {'owns': set(), 'uses': set()})

    # 筛选核心类（struct、enum、trait）
    core_classes = set()
    for node_path, node_info in nodes.items():
        node_type = node_info['type']
        # 确保只包含真正的类（struct、enum、trait），不包含方法
        if any(t in node_type for t in ['struct', 'enum', 'trait']) and 'fn' not in node_type:
            # 进一步检查路径，确保不包含方法名（通常是小写开头）
            parts = node_path.split('::')
            if parts:
                last_part = parts[-1]
                # 方法名通常是小写开头，而类名是大写开头
                if last_part and last_part[0].isupper():
                    core_classes.add(node_path)

    for edge in edges:
        source = edge['source']
        target = edge['target']
        relation = edge['relation']

        # 只关注类之间的依赖，忽略方法调用
        if source in core_classes and target in core_classes:
            class_deps[source][relation].add(target)

    return class_deps


def build_package_dependencies(edges: List[Dict], nodes: Dict) -> Dict[str, Dict[str, Set[str]]]:
    """构建每个包的依赖关系。"""
    package_deps = defaultdict(lambda: {'owns': set(), 'uses': set()})

    for edge in edges:
        source = edge['source']
        target = edge['target']
        relation = edge['relation']

        source_pkg = get_top_level_module(source)
        target_pkg = get_top_level_module(target)

        if source_pkg != target_pkg:
            package_deps[source_pkg][relation].add(target_pkg)

    return package_deps


def build_file_dependencies(edges: List[Dict], nodes: Dict) -> Dict[str, Dict[str, Set[str]]]:
    """构建每个文件的依赖关系。"""
    file_deps = defaultdict(lambda: {'owns': set(), 'uses': set()})

    for edge in edges:
        source = edge['source']
        target = edge['target']
        relation = edge['relation']

        source_file = get_file_path(source)
        target_file = get_file_path(target)

        if source_file != target_file:
            file_deps[source_file][relation].add(target_file)

    return file_deps


def format_dependency_set(deps: Set[str], relation: str) -> List[str]:
    """格式化依赖集合为可读字符串列表。"""
    result = []
    for dep in sorted(deps):
        class_name = extract_class_name(dep)
        pkg = get_top_level_module(dep)
        short_pkg = pkg.replace('austrian_market_sim::', '') if pkg != 'austrian_market_sim' else 'root'
        result.append(f"  - {class_name} ({short_pkg})")
    return result


def generate_mermaid_package_graph(package_deps: Dict) -> str:
    """生成包级依赖的 mermaid 图。"""
    mermaid_code = "```mermaid\n"
    mermaid_code += "flowchart TD\n"
    
    # 添加节点
    nodes_set = set()
    for pkg, deps in package_deps.items():
        nodes_set.add(pkg)
        for relation in ['owns', 'uses']:
            for target in deps[relation]:
                nodes_set.add(target)
    
    # 节点映射
    node_map = {}
    for i, node in enumerate(nodes_set):
        short_name = node.replace('austrian_market_sim::', '') if node != 'austrian_market_sim' else 'root'
        node_map[node] = f"pkg_{i}_{short_name}"
        mermaid_code += f'    {node_map[node]}["{short_name}"]\n'
    
    # 添加边
    for pkg, deps in package_deps.items():
        source_node = node_map[pkg]
        
        # owns 关系（实线）
        for target in deps['owns']:
            target_node = node_map[target]
            mermaid_code += f"    {source_node} -->|owns| {target_node}\n"
        
        # uses 关系（虚线）
        for target in deps['uses']:
            target_node = node_map[target]
            mermaid_code += f"    {source_node} -.->|uses| {target_node}\n"
    
    mermaid_code += "```\n"
    return mermaid_code


def generate_mermaid_file_graph(file_deps: Dict) -> str:
    """生成文件级依赖的 mermaid 图。"""
    mermaid_code = "```mermaid\n"
    mermaid_code += "flowchart TD\n"
    mermaid_code += "    classDef file fill:#f9f,stroke:#333,stroke-width:2px;\n"
    
    # 添加节点
    nodes_set = set()
    for file_path, deps in file_deps.items():
        nodes_set.add(file_path)
        for relation in ['owns', 'uses']:
            for target in deps[relation]:
                nodes_set.add(target)
    
    # 节点映射
    node_map = {}
    for i, node in enumerate(nodes_set):
        short_name = node.split('/')[-1] if '/' in node else node
        node_map[node] = f"file_{i}_{short_name}"
        mermaid_code += f'    {node_map[node]}["{short_name}"]\n'
    
    # 添加边
    for file_path, deps in file_deps.items():
        source_node = node_map[file_path]
        
        # owns 关系（实线）
        for target in deps['owns']:
            target_node = node_map[target]
            mermaid_code += f"    {source_node} -->|owns| {target_node}\n"
        
        # uses 关系（虚线）
        for target in deps['uses']:
            target_node = node_map[target]
            mermaid_code += f"    {source_node} -.->|uses| {target_node}\n"
    
    mermaid_code += "```\n"
    return mermaid_code


def detect_cyclic_dependencies(deps: Dict[str, Dict[str, Set[str]]]) -> List[List[str]]:
    """检测循环依赖。"""
    # 构建依赖图
    graph = {}
    for node, deps_info in deps.items():
        graph[node] = set()
        for relation in ['owns', 'uses']:
            graph[node].update(deps_info[relation])
    
    # 检测循环
    cycles = []
    visited = set()
    rec_stack = []
    
    def dfs(node, path):
        visited.add(node)
        rec_stack.append(node)
        
        if node in graph:
            for neighbor in graph[node]:
                if neighbor not in visited:
                    dfs(neighbor, path + [neighbor])
                elif neighbor in rec_stack:
                    # 找到循环
                    cycle_start = rec_stack.index(neighbor)
                    cycle = rec_stack[cycle_start:] + [neighbor]
                    if cycle not in cycles:
                        cycles.append(cycle)
        
        rec_stack.remove(node)
    
    for node in graph:
        if node not in visited:
            dfs(node, [node])
    
    return cycles

def generate_mermaid_class_graph(class_deps: Dict, internal_nodes: Dict) -> str:
    """生成类级依赖的 mermaid 图。"""
    # 筛选核心类（struct、enum、trait）
    core_classes = {}
    for node_path, node_info in internal_nodes.items():
        node_type = node_info['type']
        if any(t in node_type for t in ['struct', 'enum', 'trait']) and 'fn' not in node_type:
            # 进一步检查路径，确保不包含方法名（通常是小写开头）
            parts = node_path.split('::')
            if parts:
                last_part = parts[-1]
                # 方法名通常是小写开头，而类名是大写开头
                if last_part and last_part[0].isupper():
                    core_classes[node_path] = extract_class_name(node_path)
    
    mermaid_code = "```mermaid\n"
    mermaid_code += "flowchart TD\n"
    mermaid_code += "    classDef struct fill:#f9f,stroke:#333,stroke-width:2px;\n"
    mermaid_code += "    classDef enum fill:#fcf,stroke:#333,stroke-width:2px;\n"
    mermaid_code += "    classDef trait fill:#cff,stroke:#333,stroke-width:2px;\n"
    
    # 节点映射
    node_map = {}
    for i, (path, name) in enumerate(core_classes.items()):
        node_id = f"class_{i}_{name}"
        node_map[path] = node_id
        mermaid_code += f"    {node_id}[\"{name}\"]\n"
    
    # 添加边
    for source_path, deps in class_deps.items():
        if source_path not in node_map:
            continue
        source_node = node_map[source_path]
        
        for relation in ['owns', 'uses']:
            for target_path in deps[relation]:
                if target_path in node_map:
                    target_node = node_map[target_path]
                    if relation == 'owns':
                        mermaid_code += f"    {source_node} -->|owns| {target_node}\n"
                    else:
                        mermaid_code += f"    {source_node} -.->|use| {target_node}\n"
    
    mermaid_code += "```\n"
    return mermaid_code

def generate_markdown_report(nodes: Dict, edges: List[Dict], output_path: str):
    """生成 Markdown 格式的依赖报告。"""
    internal_nodes, internal_edges = filter_internal_dependencies(nodes, edges)
    class_deps = build_class_dependencies(internal_edges, nodes)
    package_deps = build_package_dependencies(internal_edges, nodes)
    file_deps = build_file_dependencies(internal_edges, nodes)

    packages = defaultdict(list)
    for node_path, node_info in internal_nodes.items():
        pkg = get_top_level_module(node_path)
        node_type = node_info['type']
        if 'struct' in node_type or 'enum' in node_type or 'trait' in node_type:
            if 'fn' not in node_type:
                packages[pkg].append({
                    'name': extract_class_name(node_path),
                    'type': node_type.split(' ')[0] if ' ' in node_type else node_type,
                    'path': node_path
                })

    # 收集文件信息
    files_info = defaultdict(list)
    for node_path, node_info in internal_nodes.items():
        file_path = get_file_path(node_path)
        node_type = node_info['type']
        if 'struct' in node_type or 'enum' in node_type or 'trait' in node_type:
            if 'fn' not in node_type:
                files_info[file_path].append({
                    'name': extract_class_name(node_path),
                    'type': node_type.split(' ')[0] if ' ' in node_type else node_type
                })

    # 检测循环依赖
    file_cycles = detect_cyclic_dependencies(file_deps)
    class_cycles = detect_cyclic_dependencies(class_deps)

    with open(output_path, 'w', encoding='utf-8') as f:
        f.write("# 奥地利市场模拟器 - 架构依赖分析报告\n\n")

        f.write("## 1. 包结构概览\n\n")
        f.write("| 包名 | 包含的核心类型 |\n")
        f.write("|------|---------------|\n")
        for pkg, types in sorted(packages.items()):
            pkg_short = pkg.replace('austrian_market_sim::', '') if pkg != 'austrian_market_sim' else '根目录'
            type_names = ', '.join([t['name'] for t in types[:5]])
            if len(types) > 5:
                type_names += f" ... (共 {len(types)} 个)"
            f.write(f"| `{pkg_short}` | {type_names} |\n")
        f.write("\n")

        f.write("## 2. 文件结构概览\n\n")
        f.write("| 文件路径 | 包含的核心类型 |\n")
        f.write("|---------|---------------|\n")
        for file_path, types in sorted(files_info.items()):
            type_names = ', '.join([t['name'] for t in types[:5]])
            if len(types) > 5:
                type_names += f" ... (共 {len(types)} 个)"
            f.write(f"| `{file_path}` | {type_names} |\n")
        f.write("\n")

        f.write("## 3. 文件级别依赖关系\n\n")
        f.write("| 文件路径 | 包含依赖数 (owns) | 使用依赖数 (uses) | 总依赖数 |\n")
        f.write("|---------|-----------------|-----------------|---------|\n")
        for file_path in sorted(file_deps.keys()):
            deps = file_deps[file_path]
            owns_count = len(deps['owns'])
            uses_count = len(deps['uses'])
            total_count = owns_count + uses_count
            f.write(f"| `{file_path}` | {owns_count} | {uses_count} | {total_count} |\n")
        f.write("\n")

        f.write("## 4. 文件依赖关系图 (Mermaid)\n\n")
        f.write(generate_mermaid_file_graph(file_deps))
        f.write("\n")

        f.write("## 5. 核心类依赖详情\n\n")

        # 遍历所有核心类依赖
        for class_path in sorted(class_deps.keys()):
            class_name = extract_class_name(class_path)
            f.write(f"### {class_name}\n\n")
            deps = class_deps[class_path]

            if deps['owns']:
                f.write("**包含 (owns):**\n")
                for target in sorted(deps['owns']):
                    short_name = target.split('::')[-1]
                    f.write(f"- `{short_name}`\n")
                f.write("\n")

            if deps['uses']:
                f.write("**使用 (uses):**\n")
                for target in sorted(deps['uses']):
                    short_name = target.split('::')[-1]
                    target_pkg = get_top_level_module(target)
                    pkg_short = target_pkg.replace('austrian_market_sim::', '') if target_pkg != 'austrian_market_sim' else 'root'
                    f.write(f"- `{short_name}` ({pkg_short})\n")
                f.write("\n")

        f.write("## 6. 类依赖关系图 (Mermaid)\n\n")
        f.write(generate_mermaid_class_graph(class_deps, internal_nodes))
        f.write("\n")

        # 循环依赖分析
        f.write("## 7. 循环依赖分析\n\n")
        
        # 文件级循环依赖
        f.write("### 文件级循环依赖\n\n")
        if file_cycles:
            for i, cycle in enumerate(file_cycles, 1):
                f.write(f"**循环 {i}:**\n")
                for j, file_path in enumerate(cycle[:-1]):
                    short_name = file_path.split('/')[-1] if '/' in file_path else file_path
                    if j < len(cycle) - 2:
                        f.write(f"`{short_name}` → ")
                    else:
                        f.write(f"`{short_name}` → `{cycle[0].split('/')[-1]}`\n")
                f.write("\n")
        else:
            f.write("未检测到文件级循环依赖。\n\n")
        
        # 类级循环依赖
        f.write("### 类级循环依赖\n\n")
        if class_cycles:
            for i, cycle in enumerate(class_cycles, 1):
                f.write(f"**循环 {i}:**\n")
                for j, class_path in enumerate(cycle[:-1]):
                    class_name = extract_class_name(class_path)
                    if j < len(cycle) - 2:
                        f.write(f"`{class_name}` → ")
                    else:
                        f.write(f"`{class_name}` → `{extract_class_name(cycle[0])}`\n")
                f.write("\n")
        else:
            f.write("未检测到类级循环依赖。\n\n")

        f.write("## 8. 模块结构总结\n\n")
        f.write("```\n")
        f.write("austrian_market_sim\n")
        f.write("├── src\n")
        f.write("│   ├── entity\n")
        f.write("│   │   └── normal_distribute.rs\n")
        f.write("│   ├── logging\n")
        f.write("│   │   ├── agent_cash_log.rs\n")
        f.write("│   │   ├── agent_demand_removal_log.rs\n")
        f.write("│   │   ├── agent_range_adjustment_log.rs\n")
        f.write("│   │   ├── factory_end_of_round_log.rs\n")
        f.write("│   │   ├── factory_range_optimization_log.rs\n")
        f.write("│   │   ├── trade_log.rs\n")
        f.write("│   │   └── mod.rs\n")
        f.write("│   ├── model\n")
        f.write("│   │   ├── agent\n")
        f.write("│   │   │   ├── preference.rs\n")
        f.write("│   │   │   └── mod.rs\n")
        f.write("│   │   ├── factory\n")
        f.write("│   │   │   ├── accountant.rs\n")
        f.write("│   │   │   ├── financial_bill.rs\n")
        f.write("│   │   │   └── mod.rs\n")
        f.write("│   │   ├── market.rs\n")
        f.write("│   │   ├── math.rs\n")
        f.write("│   │   ├── product.rs\n")
        f.write("│   │   └── mod.rs\n")
        f.write("│   ├── util\n")
        f.write("│   │   └── mod.rs\n")
        f.write("│   ├── lib.rs\n")
        f.write("│   └── main.rs\n")
        f.write("```\n")

    print(f"报告已生成: {output_path}")


def install():
    """将脚本安装到 ~/.local/bin 目录下"""
    script_path = os.path.abspath(__file__)
    install_dir = os.path.expanduser("~/.local/bin")
    install_path = os.path.join(install_dir, "cargo-dep-analyzer")
    
    # 确保安装目录存在
    os.makedirs(install_dir, exist_ok=True)
    
    # 复制脚本到安装目录
    try:
        import shutil
        shutil.copy2(script_path, install_path)
        os.chmod(install_path, 0o755)  # 设置可执行权限
        print(f"成功安装到: {install_path}")
        print("你现在可以在任何目录中使用 'cargo-dep-analyzer' 命令")
    except Exception as e:
        print(f"安装失败: {e}", file=sys.stderr)
        sys.exit(1)


def main():
    # 解析命令行参数
    parser = argparse.ArgumentParser(
        description="奥地利市场模拟器依赖分析工具",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
    # 使用默认输出文件
    cargo-dep-analyzer
    
    # 指定输出文件
    cargo-dep-analyzer -o dependency_report.md
    
    # 安装到 ~/.local/bin
    cargo-dep-analyzer --install
        """
    )
    parser.add_argument(
        "-o", "--output",
        type=str,
        default="architecture_dependency_report.md",
        help="指定输出报告文件路径 (默认: architecture_dependency_report.md)"
    )
    parser.add_argument(
        "--install",
        action="store_true",
        help="将工具安装到 ~/.local/bin 目录"
    )
    args = parser.parse_args()
    
    # 处理安装命令
    if args.install:
        install()
        return

    # 检查当前目录是否存在Cargo.toml文件
    if not os.path.exists("Cargo.toml"):
        print("错误: 当前目录不存在 Cargo.toml 文件", file=sys.stderr)
        sys.exit(1)

    output_file = args.output

    print("正在执行 cargo modules dependencies 命令...")
    import subprocess
    result = subprocess.run(
        ["cargo", "modules", "dependencies"],
        capture_output=True,
        text=True,
        cwd="."
    )
    
    if result.returncode != 0:
        print(f"命令执行失败: {result.stderr}", file=sys.stderr)
        sys.exit(1)
    
    print("正在解析命令输出...")
    nodes, edges = parse_structure_output(result.stdout)
    print(f"共解析 {len(nodes)} 个节点, {len(edges)} 条边")

    internal_nodes, internal_edges = filter_internal_dependencies(nodes, edges)
    print(f"仓库内部: {len(internal_nodes)} 个节点, {len(internal_edges)} 条边")

    print("正在生成依赖报告...")
    generate_markdown_report(nodes, edges, output_file)

    print(f"完成! 报告已生成: {output_file}")


if __name__ == "__main__":
    main()
