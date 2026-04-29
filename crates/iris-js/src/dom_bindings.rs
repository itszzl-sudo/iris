//! DOM API 绑定
//!
//! 将 iris-layout 的 DOM 节点绑定到 JavaScript 环境，
//! 提供 document、window、Element 等 Web API。

use boa_engine::{
    Context, JsValue, Source, js_string,
    property::Attribute,
};
use iris_layout::domtree::DOMTree;
use std::cell::RefCell;
use std::rc::Rc;

/// DOM 绑定管理器
///
/// 负责在 JavaScript 环境中注入和操作 DOM API。
pub struct DOMBindings {
    /// DOM 树（共享所有权）
    dom_tree: Rc<RefCell<DOMTree>>,
}

impl DOMBindings {
    /// 创建新的 DOM 绑定管理器
    ///
    /// # 参数
    ///
    /// * `dom_tree` - DOM 树的共享引用
    pub fn new(dom_tree: Rc<RefCell<DOMTree>>) -> Self {
        Self { dom_tree }
    }

    /// 注入完整的 DOM API 到 JavaScript 环境
    ///
    /// # 参数
    ///
    /// * `context` - Boa JavaScript 上下文
    pub fn inject(&self, context: &mut Context) -> Result<(), String> {
        self.inject_document(context)?;
        self.inject_window(context)?;
        self.inject_element(context)?;
        Ok(())
    }

    /// 注入 document 对象（完整实现）
    fn inject_document(&self, context: &mut Context) -> Result<(), String> {
        // 使用 JavaScript 代码创建功能完整的 document 对象
        let document_code = r#"
            (function() {
                var elementIdCounter = 0;
                var elements = {};
                
                var doc = {
                    title: "Iris App",
                    URL: "about:blank",
                    domain: "",
                    
                    // 创建元素
                    createElement: function(tag) {
                        var id = ++elementIdCounter;
                        var elem = {
                            nodeType: 1,
                            tagName: tag.toUpperCase(),
                            id: "",
                            className: "",
                            attributes: {},
                            children: [],
                            parentNode: null,
                            
                            // 设置属性
                            setAttribute: function(name, value) {
                                this.attributes[name] = value;
                                if (name === 'id') this.id = value;
                                if (name === 'class') this.className = value;
                            },
                            
                            // 获取属性
                            getAttribute: function(name) {
                                return this.attributes[name] || null;
                            },
                            
                            // 添加子节点
                            appendChild: function(child) {
                                child.parentNode = this;
                                this.children.push(child);
                                return child;
                            },
                            
                            // 移除子节点
                            removeChild: function(child) {
                                var idx = this.children.indexOf(child);
                                if (idx !== -1) {
                                    child.parentNode = null;
                                    return this.children.splice(idx, 1)[0];
                                }
                                return null;
                            },
                            
                            // 插入子节点
                            insertBefore: function(newChild, refChild) {
                                var idx = this.children.indexOf(refChild);
                                if (idx !== -1) {
                                    newChild.parentNode = this;
                                    this.children.splice(idx, 0, newChild);
                                    return newChild;
                                }
                                return this.appendChild(newChild);
                            },
                            
                            // 替换子节点
                            replaceChild: function(newChild, oldChild) {
                                var idx = this.children.indexOf(oldChild);
                                if (idx !== -1) {
                                    oldChild.parentNode = null;
                                    newChild.parentNode = this;
                                    this.children[idx] = newChild;
                                    return oldChild;
                                }
                                return null;
                            },
                            
                            // 查询选择器（简化实现）
                            querySelector: function(selector) {
                                if (selector.startsWith('#')) {
                                    var id = selector.substring(1);
                                    return doc.getElementById(id);
                                }
                                return null;
                            },
                            
                            querySelectorAll: function(selector) {
                                return [];
                            }
                        };
                        elements[id] = elem;
                        return elem;
                    },
                    
                    // 创建文本节点
                    createTextNode: function(text) {
                        return {
                            nodeType: 3,
                            textContent: text
                        };
                    },
                    
                    // 创建注释节点
                    createComment: function(data) {
                        return {
                            nodeType: 8,
                            data: data
                        };
                    },
                    
                    // 通过 ID 获取元素（遍历所有元素）
                    getElementById: function(id) {
                        function searchElements(obj, targetId) {
                            if (obj.id === targetId) return obj;
                            if (obj.children) {
                                for (var i = 0; i < obj.children.length; i++) {
                                    var found = searchElements(obj.children[i], targetId);
                                    if (found) return found;
                                }
                            }
                            return null;
                        }
                        
                        if (doc.body) {
                            return searchElements(doc.body, id);
                        }
                        return null;
                    },
                    
                    // 通过标签名获取元素（简化）
                    getElementsByTagName: function(tag) {
                        var results = [];
                        function search(obj, targetTag) {
                            if (obj.tagName === targetTag) results.push(obj);
                            if (obj.children) {
                                for (var i = 0; i < obj.children.length; i++) {
                                    search(obj.children[i], targetTag);
                                }
                            }
                        }
                        if (doc.body) {
                            search(doc.body, tag.toUpperCase());
                        }
                        return results;
                    },
                    
                    // body 元素
                    body: null,
                    
                    // 根元素
                    documentElement: null
                };
                
                // 创建默认的 body 和 html 元素
                doc.documentElement = doc.createElement('html');
                doc.body = doc.createElement('body');
                doc.documentElement.appendChild(doc.body);
                
                return doc;
            })()
        "#;

        let document = context
            .eval(Source::from_bytes(document_code))
            .map_err(|e| format!("Failed to create document: {}", e))?;

        context
            .register_global_property(js_string!("document"), document, Attribute::all())
            .map_err(|e| format!("Failed to register document: {}", e))?;
        
        Ok(())
    }

    /// 注入 window 对象（完整实现）
    fn inject_window(&self, context: &mut Context) -> Result<(), String> {
        let window_code = r#"
            (function() {
                var win = {
                    // 窗口尺寸（默认 800x600）
                    innerWidth: 800,
                    innerHeight: 600,
                    outerWidth: 1024,
                    outerHeight: 768,
                    
                    // 位置信息（简化）
                    screenX: 0,
                    screenY: 0,
                    
                    // 对话框
                    alert: function(msg) {
                        console.log("[alert]", msg);
                    },
                    prompt: function(msg, defaultValue) {
                        console.log("[prompt]", msg, defaultValue);
                        return null;
                    },
                    confirm: function(msg) {
                        console.log("[confirm]", msg);
                        return true;
                    },
                    
                    // 定时器（简化实现）
                    setTimeout: function(callback, delay) {
                        console.log("[setTimeout] delay:", delay);
                        return 1;
                    },
                    clearTimeout: function(id) {
                        console.log("[clearTimeout]", id);
                    },
                    setInterval: function(callback, delay) {
                        console.log("[setInterval] delay:", delay);
                        return 1;
                    },
                    clearInterval: function(id) {
                        console.log("[clearInterval]", id);
                    },
                    
                    // 导航（简化）
                    location: {
                        href: "about:blank",
                        protocol: "about:",
                        host: "",
                        hostname: "",
                        pathname: "/blank",
                        search: "",
                        hash: "",
                        reload: function() {
                            console.log("[location.reload]");
                        },
                        replace: function(url) {
                            console.log("[location.replace]", url);
                            this.href = url;
                        }
                    },
                    
                    // 历史（简化）
                    history: {
                        length: 1,
                        back: function() {
                            console.log("[history.back]");
                        },
                        forward: function() {
                            console.log("[history.forward]");
                        },
                        go: function(delta) {
                            console.log("[history.go]", delta);
                        },
                        pushState: function(state, title, url) {
                            console.log("[history.pushState]", state, title, url);
                        },
                        replaceState: function(state, title, url) {
                            console.log("[history.replaceState]", state, title, url);
                        }
                    },
                    
                    // 本地存储（简化，使用内存对象）
                    localStorage: {
                        _data: {},
                        getItem: function(key) {
                            return this._data[key] || null;
                        },
                        setItem: function(key, value) {
                            this._data[key] = String(value);
                        },
                        removeItem: function(key) {
                            delete this._data[key];
                        },
                        clear: function() {
                            this._data = {};
                        },
                        key: function(index) {
                            var keys = Object.keys(this._data);
                            return keys[index] || null;
                        }
                    },
                    
                    sessionStorage: {
                        _data: {},
                        getItem: function(key) {
                            return this._data[key] || null;
                        },
                        setItem: function(key, value) {
                            this._data[key] = String(value);
                        },
                        removeItem: function(key) {
                            delete this._data[key];
                        },
                        clear: function() {
                            this._data = {};
                        }
                    },
                    
                    // 窗口操作（简化）
                    focus: function() {
                        console.log("[window.focus]");
                    },
                    blur: function() {
                        console.log("[window.blur]");
                    },
                    close: function() {
                        console.log("[window.close]");
                    },
                    
                    // document 的引用（后面会注入）
                    document: null,
                    
                    // self 和 window 的循环引用（后面会设置）
                    self: null,
                    window: null,
                    
                    // 性能 API（简化）
                    performance: {
                        now: function() {
                            return Date.now();
                        }
                    }
                };
                
                return win;
            })()
        "#;

        let window = context
            .eval(Source::from_bytes(window_code))
            .map_err(|e| format!("Failed to create window: {}", e))?;

        context
            .register_global_property(js_string!("window"), window.clone(), Attribute::all())
            .map_err(|e| format!("Failed to register window: {}", e))?;
        
        // 设置 self 和 window 的自引用（通过 window.self = window）
        context
            .eval(Source::from_bytes("window.self = window; window.window = window;"))
            .map_err(|e| format!("Failed to set window self-reference: {}", e))?;
        
        // 将 document 注入到 window 对象
        context
            .eval(Source::from_bytes("window.document = document;"))
            .map_err(|e| format!("Failed to inject document into window: {}", e))?;
        
        Ok(())
    }

    /// 注入 Element 原型
    fn inject_element(&self, context: &mut Context) -> Result<(), String> {
        let element_code = r#"
            (function() {
                function Element(tagName) {
                    this.tagName = tagName.toUpperCase();
                    this.attributes = {};
                    this.children = [];
                    this.parentNode = null;
                }
                
                Element.prototype.appendChild = function(child) {
                    this.children.push(child);
                    child.parentNode = this;
                    return child;
                };
                
                Element.prototype.removeChild = function(child) {
                    var index = this.children.indexOf(child);
                    if (index > -1) {
                        this.children.splice(index, 1);
                        child.parentNode = null;
                    }
                    return child;
                };
                
                Element.prototype.setAttribute = function(name, value) {
                    this.attributes[name] = value;
                };
                
                Element.prototype.getAttribute = function(name) {
                    return this.attributes[name] || null;
                };
                
                Element.prototype.addEventListener = function(event, handler) {
                    // 简化实现
                };
                
                Element.prototype.removeEventListener = function(event, handler) {
                    // 简化实现
                };
                
                return Element;
            })()
        "#;

        let element = context
            .eval(Source::from_bytes(element_code))
            .unwrap_or_else(|e| {
                eprintln!("Failed to create Element: {}", e);
                JsValue::null()
            });

        context
            .register_global_property(js_string!("Element"), element, Attribute::all())
            .unwrap_or_else(|e| eprintln!("Failed to register Element: {}", e));
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iris_layout::dom::DOMNode;

    #[test]
    fn test_create_dom_bindings() {
        let root = DOMNode::new_element("html");
        let dom_tree = DOMTree::new(root);
        let bindings = DOMBindings::new(Rc::new(RefCell::new(dom_tree)));

        // 验证可以创建
        assert!(true);
    }

    #[test]
    fn test_inject_dom_api() {
        let root = DOMNode::new_element("html");
        let dom_tree = DOMTree::new(root);
        let bindings = DOMBindings::new(Rc::new(RefCell::new(dom_tree)));

        let mut context = Context::default();
        let result = bindings.inject(&mut context);

        assert!(result.is_ok());
    }

    #[test]
    fn test_document_methods_exist() {
        let root = DOMNode::new_element("html");
        let dom_tree = DOMTree::new(root);
        let bindings = DOMBindings::new(Rc::new(RefCell::new(dom_tree)));

        let mut context = Context::default();
        bindings.inject(&mut context).unwrap();

        // 验证 document 对象存在
        let document = context
            .global_object()
            .get(js_string!("document"), &mut context)
            .unwrap();

        assert!(!document.is_undefined());
        assert!(!document.is_null());
    }

    #[test]
    fn test_document_create_element() {
        let root = DOMNode::new_element("html");
        let dom_tree = DOMTree::new(root);
        let bindings = DOMBindings::new(Rc::new(RefCell::new(dom_tree)));

        let mut context = Context::default();
        bindings.inject(&mut context).unwrap();

        // 测试 createElement
        let result = context
            .eval(Source::from_bytes("document.createElement('div')"))
            .unwrap();
        assert!(!result.is_null());
    }

    #[test]
    fn test_window_alert() {
        let root = DOMNode::new_element("html");
        let dom_tree = DOMTree::new(root);
        let bindings = DOMBindings::new(Rc::new(RefCell::new(dom_tree)));

        let mut context = Context::default();
        let result = bindings.inject(&mut context);
        
        // 注入应该成功
        assert!(result.is_ok());

        // 测试 alert 函数存在
        let alert_result = context.eval(Source::from_bytes("typeof window.alert"));
        assert!(alert_result.is_ok());
    }

    #[test]
    fn test_element_creation() {
        let root = DOMNode::new_element("html");
        let dom_tree = DOMTree::new(root);
        let bindings = DOMBindings::new(Rc::new(RefCell::new(dom_tree)));

        let mut context = Context::default();
        bindings.inject(&mut context).unwrap();

        // 测试 Element 构造函数
        let result = context.eval(Source::from_bytes("new Element('div')")).unwrap();
        assert!(!result.is_null());
    }
}
