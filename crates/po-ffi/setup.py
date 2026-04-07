from setuptools import setup

setup(
    name="protocol-orzatty",
    version="0.1.1",
    description="Protocol Orzatty P2P Python Bindings (Native FFI)",
    author="Dylan Orzatty",
    url="https://orzatty.com",
    py_modules=["po"],
    package_data={
        "": ["*.so", "*.dll", "*.dylib"]
    },
    include_package_data=True,
)
