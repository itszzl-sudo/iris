/**
 * Iris Mock Data Engine
 * 零外部依赖，内置 Mock 数据生成引擎
 * 支持：分页数据、智能推断、自定义 Schema
 */

// ============ 内置数据池 ============

const LAST_NAMES = [
  '赵', '钱', '孙', '李', '周', '吴', '郑', '王', '冯', '陈',
  '褚', '卫', '蒋', '沈', '韩', '杨', '朱', '秦', '尤', '许',
  '张', '刘', '黄', '林', '唐', '宋', '郑', '高', '郭', '马',
];

const FIRST_NAMES = [
  '伟', '芳', '娜', '敏', '静', '强', '磊', '洋', '勇', '军',
  '杰', '娟', '艳', '涛', '明', '超', '秀英', '桂英', '建华', '建国',
  '志强', '晓明', '晓红', '海燕', '桂芳',
];

const ADJECTIVES = [
  '快乐的', '聪明的', '温暖的', '闪亮的', '清新的',
  '优雅的', '活力的', '阳光的', '热情的', '宁静的',
];

const NOUNS = [
  '星空', '大海', '花园', '阳光', '微风',
  '晨曦', '晚霞', '山泉', '星辰', '月光',
];

const EMAIL_DOMAINS = [
  'example.com', 'test.com', 'demo.org', 'mail.com', 'corp.net',
  'company.cn', 'service.com', 'support.net',
];

const CITY_NAMES = [
  '北京市', '上海市', '广州市', '深圳市', '杭州市',
  '成都市', '武汉市', '南京市', '西安市', '重庆市',
  '天津市', '苏州市', '长沙市', '郑州市', '青岛市',
  '大连市', '厦门市', '宁波市', '合肥市', '福州市',
];

const STREET_NAMES = [
  '中山路', '人民路', '建设路', '解放路', '和平路',
  '长安街', '南京路', '科技路', '软件路', '创新路',
];

const PHONE_PREFIXES = ['130', '131', '132', '133', '135', '136', '137', '138', '139', '150', '151', '152', '158', '159', '166', '176', '177', '178', '186', '187', '188', '189'];

const ROLES = ['管理员', '编辑', '用户', '访客', '运营', '测试'];

const STATUS_OPTIONS = ['启用', '禁用'];

const ORDER_STATUS = ['待支付', '已支付', '已发货', '已签收', '已完成', '已取消'];

const CATEGORIES = [
  '电子产品', '服装鞋帽', '食品饮料', '家居用品',
  '图书文具', '运动户外', '美妆护肤', '母婴用品',
];

const DEPARTMENTS = [
  '技术部', '市场部', '销售部', '人事部', '财务部',
  '运营部', '产品部', '设计部', '客服部', '行政部',
];

// ============ 随机工具函数 ============

function randInt(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function pick(arr) {
  return arr[randInt(0, arr.length - 1)];
}

function pickMulti(arr, count) {
  const shuffled = [...arr].sort(() => Math.random() - 0.5);
  return shuffled.slice(0, count);
}

function randStr(length) {
  const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars.charAt(randInt(0, chars.length - 1));
  }
  return result;
}

function randDate(from, to) {
  const start = from ? new Date(from).getTime() : new Date('2024-01-01').getTime();
  const end = to ? new Date(to).getTime() : Date.now();
  return new Date(start + Math.random() * (end - start)).toISOString().split('T')[0];
}

function randDateTime(from, to) {
  const start = from ? new Date(from).getTime() : new Date('2024-01-01').getTime();
  const end = to ? new Date(to).getTime() : Date.now();
  return new Date(start + Math.random() * (end - start)).toISOString();
}

// ============ 数据生成器 ============

let idCounter = 0;

function generateValue(schema, context) {
  if (typeof schema === 'string') {
    return generateByType(schema, context);
  }
  if (typeof schema === 'number') {
    // 静态值直接返回
    return schema;
  }
  if (typeof schema === 'boolean') {
    return schema;
  }
  if (schema === null) {
    return null;
  }
  // 对象：递归生成
  if (Array.isArray(schema)) {
    return schema.map((item, i) => generateValue(item, { ...context, index: i }));
  }
  if (typeof schema === 'object') {
    const result = {};
    for (const key of Object.keys(schema)) {
      result[key] = generateValue(schema[key], context);
    }
    return result;
  }
  return schema;
}

function generateByType(type, context) {
  // 模板字符串: {{random:32}}
  if (type.startsWith('{{') && type.endsWith('}}')) {
    const inner = type.slice(2, -2);
    const [cmd, param] = inner.split(':');
    if (cmd === 'random') return randStr(parseInt(param) || 8);
    if (cmd === 'date') return randDate();
    return type;
  }

  // 嵌入模板字符串: prefix-{{random:8}}-suffix
  const templateRegex = /\{\{(\w+):(\w+)}}/g;
  if (templateRegex.test(type)) {
    templateRegex.lastIndex = 0;
    return type.replace(templateRegex, (_, cmd, param) => {
      if (cmd === 'random') return randStr(parseInt(param) || 8);
      if (cmd === 'date') return randDate();
      if (cmd === 'number') return String(randInt(parseInt(param.split(',')[0]) || 0, parseInt(param.split(',')[1]) || 100));
      return '{{' + cmd + ':' + param + '}}';
    });
  }

  const parts = type.split(':');
  const baseType = parts[0];
  const params = parts.slice(1);

  switch (baseType) {
    case 'auto-increment':
      return (context.baseId || 1) + (context.index || 0);

    case 'name':
      return pick(LAST_NAMES) + pick(FIRST_NAMES);

    case 'email':
      return 'user' + randInt(100, 999) + '@' + pick(EMAIL_DOMAINS);

    case 'phone':
      return pick(PHONE_PREFIXES) + String(randInt(10000000, 99999999));

    case 'address':
      return pick(CITY_NAMES) + pick(STREET_NAMES) + randInt(1, 999) + '号';

    case 'city':
      return pick(CITY_NAMES);

    case 'date':
      return params[0] && params[1] ? randDate(params[0], params[1]) : randDate();

    case 'datetime':
      return params[0] && params[1] ? randDateTime(params[0], params[1]) : randDateTime();

    case 'number':
      return params[0] ? randInt(parseInt(params[0]) || 0, parseInt(params[1]) || 100) : randInt(0, 100);

    case 'boolean':
      return Math.random() > 0.5;

    case 'string':
      return randStr(parseInt(params[0]) || 8);

    case 'image':
    case 'avatar':
      const seed = randInt(1, 100);
      return `https://picsum.photos/seed/${seed}/${params[0] || 100}/${params[1] || 100}`;

    case 'enum':
      return pick(params);

    case 'role':
      return pick(ROLES);

    case 'status':
      return pick(STATUS_OPTIONS);

    case 'order-status':
      return pick(ORDER_STATUS);

    case 'category':
      return pick(CATEGORIES);

    case 'department':
      return pick(DEPARTMENTS);

    case 'title':
      return pick(ADJECTIVES) + pick(NOUNS);

    case 'id':
      return 'ID-' + String(randInt(10000, 99999));

    case 'url':
      return 'https://' + pick(EMAIL_DOMAINS) + '/' + randStr(6);

    default:
      return String(type);
  }
}

// ============ Schema 推断引擎 ============

const RESOURCE_PATTERNS = {
  'user': ['id', 'name', 'email', 'phone', 'role', 'status', 'department', 'avatar', 'createdAt'],
  'users': ['id', 'name', 'email', 'phone', 'role', 'status', 'department', 'avatar', 'createdAt'],
  'order': ['id', 'orderNo', 'amount', 'status', 'customer', 'product', 'quantity', 'createdAt'],
  'orders': ['id', 'orderNo', 'amount', 'status', 'customer', 'product', 'quantity', 'createdAt'],
  'product': ['id', 'name', 'category', 'price', 'stock', 'status', 'image', 'createdAt'],
  'products': ['id', 'name', 'category', 'price', 'stock', 'status', 'image', 'createdAt'],
  'article': ['id', 'title', 'category', 'author', 'status', 'views', 'publishedAt'],
  'articles': ['id', 'title', 'category', 'author', 'status', 'views', 'publishedAt'],
  'role': ['id', 'name', 'description', 'permissions', 'userCount', 'createdAt'],
  'roles': ['id', 'name', 'description', 'permissions', 'userCount', 'createdAt'],
  'log': ['id', 'action', 'operator', 'target', 'result', 'ip', 'timestamp'],
  'logs': ['id', 'action', 'operator', 'target', 'result', 'ip', 'timestamp'],
  'message': ['id', 'content', 'sender', 'receiver', 'type', 'isRead', 'createdAt'],
  'messages': ['id', 'content', 'sender', 'receiver', 'type', 'isRead', 'createdAt'],
};

const SCHEMA_GENERATORS = {
  'id': 'auto-increment',
  'name': 'name',
  'email': 'email',
  'phone': 'phone',
  'role': 'role',
  'status': 'status',
  'department': 'department',
  'avatar': 'avatar',
  'createdAt': 'date',
  'updatedAt': 'date',
  'publishedAt': 'date',
  'timestamp': 'datetime',
  'orderNo': 'id',
  'amount': 'number:1:10000',
  'price': 'number:10:9999',
  'stock': 'number:0:1000',
  'quantity': 'number:1:100',
  'category': 'category',
  'customer': 'name',
  'author': 'name',
  'operator': 'name',
  'sender': 'name',
  'receiver': 'name',
  'title': 'title',
  'content': 'string:50',
  'description': 'string:30',
  'url': 'url',
  'image': 'image:200:200',
  'views': 'number:0:99999',
  'userCount': 'number:0:1000',
  'isRead': 'boolean',
  'ip': 'string:15',
  'action': 'string:10',
  'target': 'string:20',
  'result': 'status',
  'permissions': 'string:16',
  'sex': 'enum:男,女',
  'age': 'number:18:60',
};

function inferSchemaFromPath(path) {
  // 提取路径中最后一个有意义的资源名
  const segments = path.split('/').filter(s => s && !s.startsWith(':') && !s.startsWith('{'));
  let resourceName = segments[segments.length - 1] || 'data';

  // 单数化处理（简单规则）
  if (resourceName.endsWith('s') && !resourceName.endsWith('ss')) {
    resourceName = resourceName.slice(0, -1);
  }

  const fields = RESOURCE_PATTERNS[resourceName] || RESOURCE_PATTERNS[resourceName + 's'];
  if (fields) {
    const schema = {};
    for (const field of fields) {
      schema[field] = SCHEMA_GENERATORS[field] || 'string:8';
    }
    return schema;
  }

  // 通用推断
  return {
    id: 'auto-increment',
    name: 'name',
    status: 'status',
    createdAt: 'date',
  };
}

// ============ 主引擎 ============

export class MockEngine {
  constructor(config = {}) {
    this.config = config;
    this.customSchemas = config.endpoints || {};
  }

  /**
   * 生成 Mock 数据
   * @param {string} path - API 路径
   * @param {string} method - HTTP 方法
   * @param {object} query - 查询参数
   * @param {object} body - 请求体
   * @returns {object} 响应数据
   */
  generate(path, method, query = {}, body = null) {
    const endpoint = this._findEndpoint(path, method);
    if (endpoint) {
      return this._generateFromEndpoint(endpoint, query, path);
    }
    return this._generateByInference(path, method, query);
  }

  /**
   * 查找匹配的端点配置
   */
  _findEndpoint(path, method) {
    // 精确匹配
    let config = this.customSchemas[path] || this.customSchemas[path + '/'];
    if (config) return config[method] || config['GET'];

    // 路径参数匹配（将 :id 或 {id} 转为通配）
    for (const [pattern, methods] of Object.entries(this.customSchemas)) {
      const regex = this._pathToRegex(pattern);
      if (regex.test(path)) {
        return methods[method] || methods['GET'];
      }
    }

    return null;
  }

  _pathToRegex(pattern) {
    const escaped = pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
    const regexStr = escaped.replace(/:(\w+)/g, '([^/]+)').replace(/\{(\w+)\}/g, '([^/]+)');
    return new RegExp('^' + regexStr + '$');
  }

  /**
   * 从端点配置生成数据
   */
  _generateFromEndpoint(endpoint, query, path) {
    const dataConfig = endpoint.data || {};
    const type = dataConfig.type || (path.includes(':id') || path.match(/\/(\d+)$/) ? 'object' : 'paginated');
    const delay = endpoint.delay || this.config.delay || 0;

    if (delay > 0) {
      // 延迟在路由层处理
    }

    switch (type) {
      case 'paginated':
        return this._generatePaginated(dataConfig, query);
      case 'array':
        return this._generateArray(dataConfig);
      case 'object':
      default:
        return this._generateObject(dataConfig, path);
    }
  }

  /**
   * 生成分页数据
   */
  _generatePaginated(dataConfig, query) {
    const schema = dataConfig.schema || { id: 'auto-increment', name: 'name' };
    const pageSize = parseInt(query.pageSize) || dataConfig.pageSize || 20;
    const page = parseInt(query.page) || 1;
    const total = dataConfig.total || 100;
    const totalPages = Math.ceil(total / pageSize);
    const startIndex = (page - 1) * pageSize;
    const count = Math.min(pageSize, total - startIndex);

    // 确保至少一页数据
    const actualPageSize = count > 0 ? count : pageSize;
    const actualStart = Math.max(0, startIndex);

    const list = [];
    for (let i = 0; i < actualPageSize; i++) {
      const context = {
        index: i,
        baseId: actualStart + 1,
      };
      list.push(generateValue(schema, context));
    }

    return {
      code: 0,
      message: 'success',
      data: {
        list,
        total,
        page,
        pageSize: actualPageSize,
        totalPages,
      },
    };
  }

  /**
   * 生成数组数据
   */
  _generateArray(dataConfig) {
    const schema = dataConfig.schema || { id: 'auto-increment', name: 'name' };
    const count = dataConfig.count || dataConfig.pageSize || 10;
    const list = [];
    for (let i = 0; i < count; i++) {
      list.push(generateValue(schema, { index: i, baseId: 1 }));
    }
    return {
      code: 0,
      message: 'success',
      data: list,
    };
  }

  /**
   * 生成单条对象数据
   */
  _generateObject(dataConfig, path) {
    const schema = dataConfig.schema || { id: 1, name: '示例数据' };

    // 从路径中提取 ID（如果有）
    const idMatch = path.match(/\/(\d+)$/);
    const context = { index: 0, baseId: idMatch ? parseInt(idMatch[1]) : 1 };

    return {
      code: 0,
      message: 'success',
      data: generateValue(schema, context),
    };
  }

  /**
   * 通过智能推断生成数据
   */
  _generateByInference(path, method, query) {
    const isList = method === 'GET' && !path.match(/\/(\d+)$/);
    const isSingle = method === 'GET' && !!path.match(/\/(\d+)$/);

    if (isList && query.page !== undefined) {
      // 分页列表
      const schema = inferSchemaFromPath(path);
      return this._generatePaginated({ schema, pageSize: 20, total: 100 }, query);
    }

    if (isList) {
      const schema = inferSchemaFromPath(path);
      return this._generateArray({ schema, count: 10 });
    }

    if (isSingle) {
      return {
        code: 0,
        message: 'success',
        data: generateValue(inferSchemaFromPath(path), { index: 0, baseId: 1 }),
      };
    }

    // POST/PUT/DELETE 返回通用成功
    if (['POST', 'PUT', 'PATCH'].includes(method)) {
      return {
        code: 0,
        message: '操作成功',
        data: { id: randInt(100, 999) },
      };
    }

    if (method === 'DELETE') {
      return {
        code: 0,
        message: '删除成功',
        data: null,
      };
    }

    return { code: 0, message: 'success', data: null };
  }

  /**
   * 检查是否为 mock 配置路径
   */
  hasEndpoint(path, method) {
    if (this.customSchemas[path]) {
      return !!this.customSchemas[path][method] || !!this.customSchemas[path]['GET'];
    }
    return this._findEndpoint(path, method) !== null;
  }
}
