import re

# 读取result.txt文件
with open("result.txt", "r", encoding="utf-8") as f:
    content = f.read()

# 去除ANSI转义序列
ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[.*?[a-zA-Z])')
content = ansi_escape.sub('', content)

# 保存清理后的内容到文件，以便查看
with open("clean_result.txt", "w", encoding="utf-8") as f:
    f.write(content)

# 测试文件名匹配
print("Testing file name extraction...")
file_match = re.search(r"`- unit: (.+?) \(@\d+\)", content)
if file_match:
    print(f"File name: {file_match.group(1)}")
else:
    print("File name not found")

# 测试函数匹配
print("\nTesting function extraction...")
function_pattern = r"function: (.+?) \(@(\d+)\)"
functions = re.findall(function_pattern, content)
print(f"Found {len(functions)} functions")
if functions:
    print(f"First function: {functions[0]}")

# 测试圈复杂度匹配
print("\nTesting cyclomatic complexity extraction...")
cyclomatic_pattern = r"cyclomatic.*?sum: (\d+)"
cyclomatics = re.findall(cyclomatic_pattern, content, re.DOTALL)
print(f"Found {len(cyclomatics)} cyclomatic complexity values")
if cyclomatics:
    print(f"First cyclomatic complexity: {cyclomatics[0]}")

# 测试代码行数匹配
print("\nTesting lines of code extraction...")
loc_pattern = r"loc.*?sloc: (\d+)"
lines = re.findall(loc_pattern, content, re.DOTALL)
print(f"Found {len(lines)} lines of code values")
if lines:
    print(f"First lines of code: {lines[0]}")

# 打印文件前50行，以便了解格式
print("\nFirst 50 lines of clean content:")
print("\n".join(content.split("\n")[:50]))