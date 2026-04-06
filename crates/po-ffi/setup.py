from setuptools import setup, Distribution

class BinaryDistribution(Distribution):
    """Distribution which always forces a binary package with platform name"""
    def has_ext_modules(self):
        return True

setup(
    name="protocol-orzatty",
    version="0.1.0",
    description="Protocol Orzatty P2P Python Bindings (Native FFI)",
    author="Dylan Orzatty",
    url="https://orzatty.com",
    py_modules=["po"],
    package_data={
        "": ["*.so", "*.dll", "*.dylib"]
    },
    include_package_data=True,
    distclass=BinaryDistribution,
)
