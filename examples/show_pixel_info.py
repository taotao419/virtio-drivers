from PIL import Image

# 打开图像文件
image = Image.open("pic2.jpg")

# 获取图像的宽度和高度
width, height = image.size

# 遍历每个像素点
for y in range(height):
    for x in range(width):
        # 获取指定位置像素的 RGB 值
        r, g, b = image.getpixel((x, y))

        # 输出 RGB 值
        print(f"Pixel at ({x}, {y}) - R: {r}, G: {g}, B: {b}")