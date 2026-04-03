import sys

def convert_to_int4(onnx_path, output_path):
    print(f"Converting {onnx_path} to INT4 {output_path}")
    # Placeholder: thực hiện lượng tử hóa
    # Có thể dùng llama.cpp hoặc gguf tools

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: convert_to_int4 <input.onnx> <output.gguf>")
        sys.exit(1)
    convert_to_int4(sys.argv[1], sys.argv[2])