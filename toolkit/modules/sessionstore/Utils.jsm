/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

"use strict";

this.EXPORTED_SYMBOLS = ["Utils"];

ChromeUtils.import("resource://gre/modules/Services.jsm", this);
ChromeUtils.import("resource://gre/modules/XPCOMUtils.jsm", this);

ChromeUtils.defineModuleGetter(this, "NetUtil",
                               "resource://gre/modules/NetUtil.jsm");
XPCOMUtils.defineLazyServiceGetter(this, "serializationHelper",
                                   "@mozilla.org/network/serialization-helper;1",
                                   "nsISerializationHelper");
XPCOMUtils.defineLazyGetter(this, "SERIALIZED_SYSTEMPRINCIPAL", function() {
  return Utils.serializePrincipal(Services.scriptSecurityManager.getSystemPrincipal());
});

function debug(msg) {
  Services.console.logStringMessage("Utils: " + msg);
}

this.Utils = Object.freeze({
  get SERIALIZED_SYSTEMPRINCIPAL() { return SERIALIZED_SYSTEMPRINCIPAL; },

  makeURI(url) {
    return Services.io.newURI(url);
  },

  makeInputStream(data) {
    if (typeof data == "string") {
      let stream = Cc["@mozilla.org/io/string-input-stream;1"].
                   createInstance(Ci.nsISupportsCString);
      stream.data = data;
      return stream; // XPConnect will QI this to nsIInputStream for us.
    }

    let stream = Cc["@mozilla.org/io/string-input-stream;1"].
                 createInstance(Ci.nsISupportsCString);
    stream.data = data.content;

    if (data.headers) {
      let mimeStream = Cc["@mozilla.org/network/mime-input-stream;1"]
          .createInstance(Ci.nsIMIMEInputStream);

      mimeStream.setData(stream);
      for (let [name, value] of data.headers) {
        mimeStream.addHeader(name, value);
      }
      return mimeStream;
    }

    return stream; // XPConnect will QI this to nsIInputStream for us.
  },

  serializeInputStream(aStream) {
    let data = {
      content: NetUtil.readInputStreamToString(aStream, aStream.available()),
    };

    if (aStream instanceof Ci.nsIMIMEInputStream) {
      data.headers = new Map();
      aStream.visitHeaders((name, value) => {
        data.headers.set(name, value);
      });
    }

    return data;
  },

  /**
   * Returns true if the |url| passed in is part of the given root |domain|.
   * For example, if |url| is "www.mozilla.org", and we pass in |domain| as
   * "mozilla.org", this will return true. It would return false the other way
   * around.
   */
  hasRootDomain(url, domain) {
    let host;

    try {
      host = this.makeURI(url).host;
    } catch (e) {
      // The given URL probably doesn't have a host.
      return false;
    }

    let index = host.indexOf(domain);
    if (index == -1)
      return false;

    if (host == domain)
      return true;

    let prevChar = host[index - 1];
    return (index == (host.length - domain.length)) &&
           (prevChar == "." || prevChar == "/");
  },

  shallowCopy(obj) {
    let retval = {};

    for (let key of Object.keys(obj)) {
      retval[key] = obj[key];
    }

    return retval;
  },

  /**
   * Serialize principal data.
   *
   * @param {nsIPrincipal} principal The principal to serialize.
   * @return {String} The base64 encoded principal data.
   */
  serializePrincipal(principal) {
    let serializedPrincipal = null;

    try {
      if (principal) {
        serializedPrincipal = serializationHelper.serializeToString(principal);
      }
    } catch (e) {
      debug(`Failed to serialize principal '${principal}' ${e}`);
    }

    return serializedPrincipal;
  },

  /**
   * Deserialize a base64 encoded principal (serialized with
   * Utils::serializePrincipal).
   *
   * @param {String} principal_b64 A base64 encoded serialized principal.
   * @return {nsIPrincipal} A deserialized principal.
   */
  deserializePrincipal(principal_b64) {
    if (!principal_b64)
      return null;

    try {
      let principal = serializationHelper.deserializeObject(principal_b64);
      principal.QueryInterface(Ci.nsIPrincipal);
      return principal;
    } catch (e) {
      debug(`Failed to deserialize principal_b64 '${principal_b64}' ${e}`);
    }
    return null;
  }
});
