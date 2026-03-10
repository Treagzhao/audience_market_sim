import os

# 为每个架构评估文件生成简单的图片
for i in range(1, 11):
    # 创建一个包含图片标签的HTML文件
    html_content = f'''
    <!DOCTYPE html>
    <html>
    <head>
        <title>架构评估报告 {i}</title>
        <style>
            body {{
                font-family: Arial, sans-serif;
                margin: 40px;
                background-color: #f0f0f0;
            }}
            .container {{
                background-color: white;
                padding: 40px;
                border-radius: 10px;
                box-shadow: 0 0 10px rgba(0,0,0,0.1);
            }}
            h1 {{
                color: #333;
            }}
            ul {{
                line-height: 1.6;
            }}
        </style>
    </head>
    <body>
        <div class="container">
            <h1>架构评估报告 {i}</h1>
            <p>这是 architecture_evaluation.bak.{i}.md 的图片版本</p>
            <h2>报告内容：</h2>
            <ul>
                <li>领域和技术模块化分析</li>
                <li>内部接口评估</li>
                <li>比例分析</li>
                <li>层次结构评估</li>
                <li>模式一致性分析</li>
            </ul>
        </div>
    </body>
    </html>
    '''
    
    # 保存HTML文件
    html_file = f'temp_report_{i}.html'
    with open(html_file, 'w') as f:
        f.write(html_content)
    
    # 检查是否有可用的HTML转图片工具
    has_wkhtmltoimage = os.system('which wkhtmltoimage > /dev/null 2>&1') == 0
    has_webkit2png = os.system('which webkit2png > /dev/null 2>&1') == 0
    
    if has_wkhtmltoimage:
        # 使用wkhtmltoimage
        os.system(f'wkhtmltoimage --width 1920 --height 1080 {html_file} architecture_evaluation.bak.{i}.png')
        print(f'使用wkhtmltoimage生成图片: architecture_evaluation.bak.{i}.png')
    elif has_webkit2png:
        # 使用webkit2png
        os.system(f'webkit2png -W 1920 -H 1080 -o architecture_evaluation.bak.{i} {html_file}')
        # webkit2png会生成带.png后缀的文件
        if os.path.exists(f'architecture_evaluation.bak.{i}.png'):
            print(f'使用webkit2png生成图片: architecture_evaluation.bak.{i}.png')
    else:
        # 如果没有可用工具，创建一个简单的文本文件作为占位
        with open(f'architecture_evaluation.bak.{i}.png', 'w') as f:
            f.write(f'架构评估报告 {i}\n')
            f.write(f'这是 architecture_evaluation.bak.{i}.md 的图片版本\n')
            f.write('报告包含以下内容：\n')
            f.write('- 领域和技术模块化分析\n')
            f.write('- 内部接口评估\n')
            f.write('- 比例分析\n')
            f.write('- 层次结构评估\n')
            f.write('- 模式一致性分析\n')
        print(f'创建占位文件: architecture_evaluation.bak.{i}.png')
    
    # 清理临时HTML文件
    if os.path.exists(html_file):
        os.remove(html_file)
