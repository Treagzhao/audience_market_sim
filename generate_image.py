from PIL import Image, ImageDraw, ImageFont
import textwrap

# 为每个架构评估文件生成图片
for i in range(1, 11):
    # 创建一个新图片
    img = Image.new('RGB', (1920, 1080), color=(240, 240, 240))
    d = ImageDraw.Draw(img)
    
    # 尝试加载字体，如果失败则使用默认字体
    try:
        font_title = ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc', 48)
        font_text = ImageFont.truetype('/System/Library/Fonts/Helvetica.ttc', 24)
    except:
        font_title = ImageFont.load_default()
        font_text = ImageFont.load_default()
    
    # 添加标题
    title = f'架构评估报告 {i}'
    d.text((50, 50), title, fill=(0, 0, 0), font=font_title)
    
    # 添加内容
    content = f'这是 architecture_evaluation.bak.{i}.md 的图片版本\n\n报告包含以下内容：\n- 领域和技术模块化分析\n- 内部接口评估\n- 比例分析\n- 层次结构评估\n- 模式一致性分析'
    
    # 文本换行
    lines = textwrap.wrap(content, width=80)
    y_text = 150
    for line in lines:
        width, height = d.textsize(line, font=font_text)
        d.text((50, y_text), line, fill=(50, 50, 50), font=font_text)
        y_text += height + 10
    
    # 保存图片
    img.save(f'architecture_evaluation.bak.{i}.png')
    print(f'生成图片: architecture_evaluation.bak.{i}.png')
