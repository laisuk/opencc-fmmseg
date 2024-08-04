from opencc_rs import OpenCC

# Example usage
opencc = OpenCC("s2t")
result = opencc.convert("你好，世界，“编程快乐”！", True)
print(result)
