angular.module('app', ['ngMaterial']);

angular.module('app').component('wordnet', {
    templateUrl: 'static/wordnet.html',
    controller: function($scope) {
        $scope.synsets = [];
        $scope.display = {
            show_wn31: true
        };
    }
});

angular.module('app').controller('SearchController',
        function($scope, $http) {
    var self = this;
    self.selectedItemChange = function(item) {
        if(item) {
            $http.get("/json/lemma/"+item.item).then(
                function(result) {
                    $scope.$parent.synsets = result.data;
                }, function(response) {
                    console.log(response.data);
                });
        } else {
            $scope.$parent.synsets = [];
        }
    };

    self.querySearch = function (query) {
        return $http.get("/autocomplete/lemma/"+ query).then(
            function(result) {
                return result.data;
            }, function(response) {
                console.log(response.data);
            });
        };
});

angular.module('app').component('synset', {
        templateUrl: '/static/synset.html',
        bindings: {
            synset: '=',
            display: '<'
        }
    });

angular.module('app').component('synset2', {
    templateUrl: '/static/synset.html',
    bindings: {
        target: '<',
        display: '<'
    },
    controller: function($http) {
        var ctrl = this;
        ctrl.synset = {};
        $http.get("/json/wn31/" + ctrl.target).then(
            function(response) {
                ctrl.synset = response.data[0];
            },
            function(response) {
                console.log(response.data);
            });
    }
});

angular.module('app').component('relation', {
        templateUrl: '/static/relation.html',
        bindings: {
            fullname: '@',
            relation: '@',
            relations: '=',
            display: '<'
//        },
//        controller: function($scope,$http) {
//            console.log("Loading synsets");
//            for(r = 0; r < this.relations.length; r++) {
//                var relation = this.relations[r];
//                console.log(relation);
//                $http.get("/json/wn31/" + relation.target).then(
//                    function(response) {
//                        console.log(response.data);
//                        relation.synset = response.data[0];
//                        console.log(relation);
//                    },
//                    function(response) {
//                        console.log(response.data);
//                    });
//            }
        }

    });


angular.module('app').filter('isAdjective', function() {
    return function(items) {
        var filtered = [];
        for(var i = 0; i < items.length; i++) {
            var item = items[i];
            if(item.pos === 'a' || item.pos === 's') {
                filtered.push(item);
            }
        }
        return filtered;
    }
});
